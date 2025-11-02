use std::fs::File;
use std::path::Path;

use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error as SymphoniaError,
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions,
};

use symphonia::default::get_probe;

pub fn decode_audio_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<f32>, u32, u16), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let probed = get_probe()
        .format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.sample_rate.is_some())
        .ok_or("no audio track found")?
        .clone();

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("track has no sample rate")?;

    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let mut pcm_samples = Vec::new();
    let mut sample_buffer_opt: Option<SampleBuffer<f32>> = None;

    // Decode all packets into PCM samples.
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break, // reached EOF
            Err(e) => return Err(format!("decode error: {:?}", e).into()),
        };

        match decoder.decode(&packet) {
            Ok(decoded) => {
                if sample_buffer_opt.is_none() {
                    let spec = *decoded.spec();
                    sample_buffer_opt = Some(SampleBuffer::<f32>::new(
                        decoded.capacity() as u64, // Use a generous capacity
                        spec,
                    ));
                }

                if let Some(sample_buffer) = sample_buffer_opt.as_mut() {
                    sample_buffer.copy_interleaved_ref(decoded);
                    pcm_samples.extend_from_slice(sample_buffer.samples());
                }
            }
            Err(SymphoniaError::DecodeError(_)) => {
                // Recoverable decode error, just skip the frame
                continue;
            }
            Err(SymphoniaError::IoError(_)) => break, // EOF reached during decode
            Err(e) => return Err(format!("decoding failed: {:?}", e).into()),
        }
    }

    Ok((pcm_samples, sample_rate, channels))
}
