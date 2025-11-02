use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::codecs::Decoder;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};

use cpal::Stream;

pub mod decoder;
pub mod player;

pub struct AudioData {
    pub samples: VecDeque<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub is_seeking: bool,
    pub is_finished: bool,
    pub volume: f32,
    pub total_samples_played: u64,
    pub min_buffer_threshold: usize,
    pub format_reader: Option<Box<dyn FormatReader + Send>>,
    pub decoder: Option<Box<dyn Decoder + Send>>,
    pub track_id: u32,
}

pub struct AudioPlayer {
    pub state: Arc<Mutex<AudioData>>,
    pub cpal_stream: Option<Stream>,
}

impl AudioPlayer {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
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

        let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;
        let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let track_id = track.id;

        let decoder = symphonia::default::get_codecs().make(
            &track.codec_params,
            &symphonia::core::codecs::DecoderOptions::default(),
        )?;

        let audio_data = AudioData {
            samples: VecDeque::new(),
            sample_rate,
            channels,
            is_seeking: false,
            is_finished: false,
            volume: 1.0,
            total_samples_played: 0,
            min_buffer_threshold: 4096,
            format_reader: Some(format_reader),
            decoder: Some(decoder),
            track_id,
        };

        Ok(Self {
            state: Arc::new(Mutex::new(audio_data)),
            cpal_stream: None,
        })
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no default device");

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
                    for sample in data.iter_mut() {
                        *sample = audio_data.samples.pop_front().unwrap_or(0.0) * audio_data.volume;
                    }
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None
        )?;

        stream.play()?;
        self.cpal_stream = Some(stream);

        let state_clone_2 = self.state.clone();
        std::thread::spawn(move || {
            Self::decode_loop(state_clone_2);
        });

        Ok(())
    }

    fn decode_loop(state: Arc<Mutex<AudioData>>) {
        loop {
            let mut audio_data = match state.lock() {
                Ok(data) => data,
                Err(_) => return,
            };

            if audio_data.is_finished || audio_data.samples.len() > audio_data.min_buffer_threshold {
                drop(audio_data);
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }

            let mut format_reader = match audio_data.format_reader.take() {
                Some(reader) => reader,
                None => return,
            };
            let mut decoder = match audio_data.decoder.take() {
                Some(decoder) => decoder,
                None => return,
            };
            let track_id = audio_data.track_id;

            let packet = match format_reader.next_packet() {
                Ok(packet) => packet,
                Err(_) => {
                    audio_data.is_finished = true;
                    audio_data.format_reader = Some(format_reader);
                    audio_data.decoder = Some(decoder);
                    break;
                }
            };

            if packet.track_id() != track_id {
                audio_data.format_reader = Some(format_reader);
                audio_data.decoder = Some(decoder);
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let mut sample_buffer = symphonia::core::audio::SampleBuffer::<f32>::new(
                        decoded.capacity() as u64,
                        *decoded.spec(),
                    );
                    sample_buffer.copy_interleaved_ref(decoded);
                    audio_data.samples.extend(sample_buffer.samples());
                }
                Err(e) => {
                    eprintln!("Decode error: {}", e);
                }
            }
            audio_data.format_reader = Some(format_reader);
            audio_data.decoder = Some(decoder);
        }
    }
}
