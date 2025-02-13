use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use enigo::{Enigo, Keyboard, Settings};

// Helper: convert i16 samples to f32 in [-1.0, 1.0]
fn convert_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
}

// Helper: convert u16 samples to f32 (centered at zero)
fn convert_u16_to_f32(samples: &[u16]) -> Vec<f32> {
    samples.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a shared buffer to store f32 samples.
    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

    // Set up the default host and input device using cpal.
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No input device available");
    println!("Input device: {}", device.name()?);

    // Use the device's default input configuration.
    let config = device.default_input_config()?;
    println!("Default input config: {:?}", config);
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let buffer_clone = audio_buffer.clone();
    // Build the input stream; handle different sample formats.
    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                let mut buf = buffer_clone.lock().unwrap();
                buf.extend_from_slice(data);
            },
            move |err| eprintln!("Stream error: {:?}", err),
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                let mut buf = buffer_clone.lock().unwrap();
                let converted = convert_i16_to_f32(data);
                buf.extend(converted);
            },
            move |err| eprintln!("Stream error: {:?}", err),
            None,
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                let mut buf = buffer_clone.lock().unwrap();
                let converted = convert_u16_to_f32(data);
                buf.extend(converted);
            },
            move |err| eprintln!("Stream error: {:?}", err),
            None,
        )?,
        sample_format => panic!("Unsupported sample format '{sample_format}'")
    };

    // Start recording.
    stream.play()?;
    println!("Recording audio for 5 seconds...");
    thread::sleep(Duration::from_secs(5));
    // Stop recording by dropping the stream.
    drop(stream);
    println!("Recording stopped.");

    // Retrieve the recorded audio samples.
    let audio_data = {
        let buf = audio_buffer.lock().unwrap();
        buf.clone()
    };
    println!("Captured {} samples", audio_data.len());

    // Initialize the Whisper model.
    let model_path = "models/ggml-base.en.bin";
    let ctx = whisper_rs::WhisperContext::new_with_params(
        model_path,
        whisper_rs::WhisperContextParameters::default()
    )?;
    let mut state = ctx.create_state()?;

    // Create parameters for full transcription
    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::default());
    params.set_print_progress(true);
    params.set_print_timestamps(true);
    params.set_language(Some("en"));

    // Run transcription (full mode) on the recorded samples
    state.full(params, &audio_data)?;

    // Retrieve transcription segments and combine their text
    let num_segments = state.full_n_segments()?;
    let mut transcription = String::new();
    
    for i in 0..num_segments {
        transcription.push_str(&state.full_get_segment_text(i)?);
        transcription.push(' ');
    }
    println!("Transcription: {}", transcription);

    // Use enigo to simulate typing the transcribed text into the active window.
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.text(&transcription)?;
    println!("Transcription sent as simulated keystrokes.");

    Ok(())
}
