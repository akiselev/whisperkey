extern crate ctranslate2_sys;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::path::PathBuf;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use hound::{WavWriter, WavSpec};

use enigo::{Enigo, Keyboard, Settings};

use nnnoiseless::DenoiseState;

// Helper: convert i16 samples to f32 in [-1.0, 1.0]
fn convert_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
}

// Helper: convert u16 samples to f32 (centered at zero)
fn convert_u16_to_f32(samples: &[u16]) -> Vec<f32> {
    samples.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect()
}

// Add this helper function after your existing convert functions
fn denoise_audio(audio_data: &[f32], channels: u16, sample_rate: u32) -> Vec<f32> {
    // RNNoise expects 48kHz mono audio, so we'll need to handle that
    let mut denoiser = DenoiseState::new();
    let frame_size = DenoiseState::FRAME_SIZE;
    let mut denoised = Vec::with_capacity(audio_data.len());
    
    // If stereo, convert to mono first
    let mono_audio: Vec<f32> = if channels == 2 {
        audio_data.chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        audio_data.to_vec()
    };

    // Process audio in frames
    for chunk in mono_audio.chunks(frame_size) {
        let mut frame = vec![0.0; frame_size];
        frame[..chunk.len()].copy_from_slice(chunk);
        
        let mut output = vec![0.0; frame_size];
        denoiser.process_frame(&mut output, &frame);
        
        denoised.extend_from_slice(&output[..chunk.len()]);
    }

    denoised
}

fn save_wav_file(
    audio_data: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all("output")?;

    // Generate timestamp for filename
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    
    let wav_path = PathBuf::from(format!("output/{}.wav", timestamp));

    // Set up WAV specifications
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    // Create and write to WAV file
    let mut writer = WavWriter::create(&wav_path, spec)?;
    
    for &sample in audio_data {
        writer.write_sample(sample)?;
    }
    
    writer.finalize()?;
    println!("Audio saved to: {}", wav_path.display());
    
    Ok(wav_path)
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

    // Store these for WAV file creation
    let sample_rate = stream_config.sample_rate.0;
    let channels = stream_config.channels;

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
    drop(stream);
    println!("Recording stopped.");

    // After recording, save to WAV file
    let audio_data = {
        let buf = audio_buffer.lock().unwrap();
        buf.clone()
    };
    println!("Captured {} samples", audio_data.len());

    // Denoise the audio before saving and transcribing
    println!("Denoising audio...");
    let denoised_audio = denoise_audio(&audio_data, channels, sample_rate);
    
    // Save the denoised audio data to a WAV file
    let wav_path = save_wav_file(&denoised_audio, sample_rate, channels)?;
    println!("Saved denoised audio to: {}", wav_path.display());

    // Initialize Whisper with optimized parameters
    let model_path = "models/ggml-base.en.bin";
    let mut context_params = whisper_rs::WhisperContextParameters::default();
    context_params.use_gpu(true);
    
    let ctx = whisper_rs::WhisperContext::new_with_params(
        model_path,
        context_params
    )?;
    let mut state = ctx.create_state()?;

    // Optimize parameters for better transcription of longer audio
    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::default());
    
    // Essential parameters for better transcription
    params.set_n_threads(4);
    params.set_language(Some("en"));
    params.set_translate(false);
    
    // Audio context parameters
    params.set_n_max_text_ctx(16384); // Maximum context size
    params.set_no_context(false);     // Use context from previous text
    
    // Timestamp and segmentation parameters
    params.set_token_timestamps(true);
    params.set_single_segment(false);  // Allow multiple segments
    params.set_max_len(0);            // No maximum segment length
    params.set_max_tokens(0);         // No token limit per segment
    
    // Quality and filtering parameters
    params.set_temperature(0.0);      // Reduce randomness in output
    params.set_entropy_thold(2.4);    // Default compression ratio threshold
    params.set_logprob_thold(-1.0);   // Default log probability threshold
    params.set_no_speech_thold(0.6);  // Higher threshold for speech detection
    
    // Text processing parameters
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);
    
    // Debug options
    params.set_print_progress(true);
    params.set_print_timestamps(true);

    println!("Starting transcription...");
    
    // Use denoised audio for transcription
    state.full(params, &denoised_audio)?;

    // Retrieve and combine all segments with better handling
    let num_segments = state.full_n_segments()?;
    let mut transcription = String::new();
    
    if num_segments == 0 {
        println!("No segments were transcribed. Check if audio input is working properly.");
    }
    
    for i in 0..num_segments {
        let segment_text = state.full_get_segment_text(i)?;
        if !segment_text.trim().is_empty() {
            if !transcription.is_empty() {
                transcription.push(' ');
            }
            transcription.push_str(segment_text.trim());
            
            // Print segment details for debugging
            let t0 = state.full_get_segment_t0(i)?;
            let t1 = state.full_get_segment_t1(i)?;
            println!("Segment {}: {} ms -> {} ms: {}", i, t0, t1, segment_text.trim());
        }
    }

    if transcription.is_empty() {
        println!("Warning: No text was transcribed. Check audio input and model.");
    } else {
        println!("\nFinal Transcription: {}", transcription);
        
        // Use enigo to simulate typing the transcribed text
        let mut enigo = Enigo::new(&Settings::default())?;
        enigo.text(&transcription)?;
        println!("Transcription sent as simulated keystrokes.");
    }

    Ok(())
}
