use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn play(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;

    let config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let sample_iter = Arc::new(Mutex::new(samples.into_iter()));
    let iter_clone = Arc::clone(&sample_iter);

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if let Ok(mut iter_guard) = iter_clone.lock() {
                for out in data.iter_mut() {
                    *out = iter_guard.next().unwrap_or(0.0);
                }
            }
        },
        move |err| eprintln!("Stream error: {}", err),
        Some(Duration::from_millis(2000))
    )?;

    stream.play()?;

    loop {
        if let Ok(iter) = sample_iter.lock() {
            if iter.size_hint().0 == 0 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
