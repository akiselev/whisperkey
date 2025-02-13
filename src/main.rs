// main.rs

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};

use enigo::{Enigo, KeyboardControllable};

use whisper_rs::WhisperState; // assumes the whisper-rs crate is added as a dependency

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
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                let mut buf = buffer_clone.lock().unwrap();
                let converted = convert_i16_to_f32(data);
                buf.extend(converted);
            },
            move |err| eprintln!("Stream error: {:?}", err),
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                let mut buf = buffer_clone.lock().unwrap();
                let converted = convert_u16_to_f32(data);
                buf.extend(converted);
            },
            move |err| eprintln!("Stream error: {:?}", err),
        )?,
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
    // (Make sure the model binary exists at the specified path.)
    let model_path = "models/ggml-base.en.bin";
    let mut whisper_state =
        WhisperState::new(model_path).expect("Failed to initialize whisper model");

    // Run transcription (full mode) on the recorded samples.
    // (Note: the model expects a specific sample rate; ensure your recording matches it.)
    whisper_state
        .full(&audio_data)
        .expect("Transcription failed");

    // Retrieve transcription segments and combine their text.
    let segments = whisper_state.get_segments();
    let transcription: String = segments
        .into_iter()
        .map(|seg| seg.text)
        .collect::<Vec<String>>()
        .join(" ");
    println!("Transcription: {}", transcription);

    // Use enigo to simulate typing the transcribed text into the active window.
    let mut enigo = Enigo::new();
    enigo.text(&transcription);
    println!("Transcription sent as simulated keystrokes.");

    Ok(())
}
