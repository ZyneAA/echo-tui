use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{FftPlanner, num_complex::Complex};
use symphonia::core::codecs::Decoder;
use symphonia::core::formats::FormatReader;

pub mod metadata;
pub mod song;

#[derive(Clone, Default)]
pub struct DurationInfo {
    pub readable: String,
    pub seconds: u64,
}

#[derive(Default)]
pub struct AudioData {
    pub samples: VecDeque<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub file_size: String,
    pub duration: DurationInfo,
    pub host: String,

    pub is_seeking: bool,
    pub is_finished: bool,
    pub is_pause: bool,
    pub volume: f32,

    pub total_samples_played: u64,
    pub min_buffer_threshold: usize,

    pub format_reader: Option<Box<dyn FormatReader + Send>>,
    pub decoder: Option<Box<dyn Decoder + Send>>,
    pub track_id: u32,

    pub fft_state: Vec<f32>,
}

pub struct AudioPlayer {
    pub state: Arc<Mutex<AudioData>>,
    pub cpal_stream: Option<Stream>,
}

impl AudioPlayer {
    pub fn bad() -> Self {
        let audio_data = AudioData::default();
        Self {
            state: Arc::new(Mutex::new(audio_data)),
            cpal_stream: None,
        }
    }

    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let file_size = human_readable_size(file.metadata()?.len());
        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

        let probed = symphonia::default::get_probe().format(
            &Default::default(),
            mss,
            &symphonia::core::formats::FormatOptions::default(),
            &symphonia::core::meta::MetadataOptions::default(),
        )?;

        let format_reader = probed.format;
        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or("No audio track found")?
            .clone();

        let duration = get_audio_duration(&track);

        let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let track_id = track.id;

        let decoder = symphonia::default::get_codecs().make(
            &track.codec_params,
            &symphonia::core::codecs::DecoderOptions::default(),
        )?;

        let audio_data = AudioData {
            samples: VecDeque::new(),
            sample_rate,
            channels,
            file_size,
            duration,
            host: String::new(),

            is_seeking: false,
            is_finished: false,
            is_pause: false,
            volume: 0.3,

            total_samples_played: 0,
            min_buffer_threshold: 4096,

            format_reader: Some(format_reader),
            decoder: Some(decoder),
            track_id,

            fft_state: vec![],
        };

        Ok(Self {
            state: Arc::new(Mutex::new(audio_data)),
            cpal_stream: None,
        })
    }

    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no default device");

        let mut state = self.state.lock().unwrap();
        state.host = device.name().unwrap();
        drop(state);

        let (channels, sample_rate) = {
            let state = self.state.lock().map_err(|_| "Mutex lock failed")?;
            (state.channels, state.sample_rate)
        };

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let state_clone = self.state.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut audio_data) = state_clone.lock() {
                    if audio_data.is_pause {
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                        return;
                    }
                    for frame in data.chunks_mut(audio_data.channels as usize) {
                        for (_, sample) in frame.iter_mut().enumerate() {
                            *sample =
                                audio_data.samples.pop_front().unwrap_or(0.0) * audio_data.volume;
                        }
                        if audio_data.is_finished != true {
                            audio_data.total_samples_played += 1;
                        }
                    }
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?;

        stream.play()?;

        self.cpal_stream = Some(stream);

        let state_clone_2 = self.state.clone();
        std::thread::spawn(move || {
            Self::decode_loop(state_clone_2);
        });

        let fft_state = self.state.clone();
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(2056);

        std::thread::spawn(move || {
            loop {
                let maybe_chunk = {
                    let data = fft_state.lock().unwrap();
                    if data.samples.len() >= 4056 {
                        // Grab a small window without draining the whole buffer
                        let chunk: Vec<f32> = data.samples.iter().take(2056).cloned().collect();
                        Some(chunk)
                    } else {
                        None
                    }
                };

                if let Some(chunk) = maybe_chunk {
                    let fft_result = Self::compute_fft(&chunk, &fft);
                    let mut data = fft_state.lock().unwrap();
                    data.fft_state = fft_result;
                }

                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        });

        Ok(())
    }

    fn compute_fft(samples: &[f32], fft: &Arc<dyn rustfft::Fft<f32>>) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .map(|&x| Complex { re: x, im: 0.0 })
            .collect();

        fft.process(&mut buffer);
        buffer.iter().map(|c| c.norm()).collect()
    }

    fn decode_loop(state: Arc<Mutex<AudioData>>) {
        loop {
            let (format_reader, decoder, track_id) = {
                let mut audio_data = state.lock().unwrap();
                if audio_data.is_finished {
                    return;
                }

                if audio_data.is_seeking {
                    audio_data.is_seeking = false;
                    audio_data.samples.clear();

                    let target_samples = audio_data.total_samples_played;

                    let time_base = audio_data
                        .format_reader
                        .as_ref()
                        .unwrap()
                        .tracks()
                        .iter()
                        .find(|t| t.id == audio_data.track_id)
                        .unwrap()
                        .codec_params
                        .time_base
                        .unwrap();

                    let seek_time = time_base.calc_time(target_samples);

                    let seek_to = symphonia::core::formats::SeekTo::Time {
                        time: seek_time,
                        track_id: Some(audio_data.track_id),
                    };

                    audio_data
                        .format_reader
                        .as_mut()
                        .unwrap()
                        .seek(symphonia::core::formats::SeekMode::Accurate, seek_to)
                        .unwrap();
                }

                if audio_data.samples.len() > audio_data.min_buffer_threshold {
                    drop(audio_data);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                (
                    audio_data.format_reader.take(),
                    audio_data.decoder.take(),
                    audio_data.track_id,
                )
            };

            let (Some(mut format_reader), Some(mut decoder)) = (format_reader, decoder) else {
                return;
            };

            let result = format_reader.next_packet().ok().and_then(|packet| {
                if packet.track_id() != track_id {
                    return None;
                }
                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let mut sample_buffer = symphonia::core::audio::SampleBuffer::<f32>::new(
                            decoded.capacity() as u64,
                            *decoded.spec(),
                        );
                        sample_buffer.copy_interleaved_ref(decoded);

                        Some(sample_buffer.samples().to_vec())
                    }
                    Err(_) => None,
                }
            });

            let mut audio_data = state.lock().unwrap();
            audio_data.format_reader = Some(format_reader);
            audio_data.decoder = Some(decoder);

            if let Some(samples) = result {
                audio_data.samples.extend(samples);
            } else {
                audio_data.is_finished = true;
                return;
            }
        }
    }
}

fn human_readable_size(size: u64) -> String {
    let units = ["b", "kb", "mb", "gb", "tb"];
    let mut size_f = size as f64;
    let mut unit = 0;

    while size_f >= 1024.0 && unit < units.len() - 1 {
        size_f /= 1024.0;
        unit += 1;
    }

    format!("{:.2}{}", size_f, units[unit])
}

fn get_audio_duration(track: &symphonia::core::formats::Track) -> DurationInfo {
    if let (Some(sample_rate), Some(n_frames)) =
        (track.codec_params.sample_rate, track.codec_params.n_frames)
    {
        let duration_secs = n_frames as f64 / sample_rate as f64;
        let hours = (duration_secs / 3600.0).floor() as u64;
        let minutes = ((duration_secs % 3600.0) / 60.0).floor() as u64;
        let seconds = (duration_secs % 60.0).round() as u64;

        let readable = if hours > 0 {
            format!("{:01}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        };

        DurationInfo {
            readable,
            seconds: duration_secs.round() as u64,
        }
    } else {
        DurationInfo {
            readable: "Unknown".into(),
            seconds: 0,
        }
    }
}

pub fn skip(
    state: Arc<Mutex<AudioData>>,
    skip_seconds: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut audio_data = state.lock().unwrap();
    audio_data.is_finished = false;

    let sample_rate = audio_data.sample_rate as f64;
    let channels = audio_data.channels as f64;
    let samples_to_skip = (skip_seconds * sample_rate * channels) as i64;

    let current_position_i64 = audio_data.total_samples_played as i64;

    let max_samples = (audio_data.duration.seconds as f64 * audio_data.sample_rate as f64) as i64;

    let mut target_samples = current_position_i64
        .saturating_add(samples_to_skip)
        .min(max_samples);

    if target_samples >= max_samples {
        target_samples = max_samples - 1;
        audio_data.is_finished = true;
    }

    if target_samples < 0 {
        target_samples = 0;
    }

    audio_data.total_samples_played = target_samples as u64;
    audio_data.is_seeking = true;

    Ok(())
}

pub fn current_timestamp(total_samples_played: u64, sample_rate: u32) -> (String, f64) {
    let seconds = (total_samples_played as f64 / sample_rate as f64).ceil();

    let hours = (seconds / 3600.0).floor() as u64;
    let minutes = ((seconds % 3600.0) / 60.0).floor() as u64;
    let secs = (seconds % 60.0).round() as u64;

    let readable = if hours > 0 {
        format!("{:01}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    };

    (readable, seconds)
}
