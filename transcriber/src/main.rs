use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use vosk::{DecodingState, Model, Recognizer};

// Define the same IPC structs as in the core library
#[derive(Debug, Serialize, Deserialize)]
struct IpcAudioChunk {
    samples: Vec<f32>,
    sample_rate: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct IpcTranscriptionResult {
    text: String,
    is_final: bool,
    confidence: Option<f32>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about = "Vosk transcription service")]
struct Args {
    /// Path to Vosk model directory
    #[clap(short, long, value_parser)]
    model_path: PathBuf,

    /// Sample rate for the recognizer (must match input audio)
    #[clap(short, long, default_value = "16000")]
    sample_rate: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    tracing::info!("Transcriber starting...");

    // Parse command line arguments
    let args = Args::parse();
    tracing::info!("Using model path: {:?}", args.model_path);
    tracing::info!("Sample rate: {}", args.sample_rate);

    // Initialize Vosk model
    tracing::info!("Loading Vosk model...");
    let model_path_str = args.model_path.to_str().ok_or_else(|| {
        let err = "Invalid model path: Path contains invalid UTF-8 characters".to_string();
        tracing::error!("{}", err);
        err
    })?;

    let model = Model::new(model_path_str).ok_or_else(|| {
        let err = format!("Failed to load Vosk model from path: {}", model_path_str);
        tracing::error!("{}", err);
        err
    })?;

    // Create recognizer
    tracing::info!("Creating recognizer with sample rate {}", args.sample_rate);
    let mut recognizer = Recognizer::new(&model, args.sample_rate as f32).ok_or_else(|| {
        let err = format!(
            "Failed to create recognizer with sample rate {}",
            args.sample_rate
        );
        tracing::error!("{}", err);
        err
    })?;

    // Set recognizer parameters
    recognizer.set_max_alternatives(1);
    recognizer.set_words(true);
    recognizer.set_partial_words(true);

    // Get stdin as a buffered reader
    let stdin = io::stdin();
    let mut reader = stdin.lock().lines();

    // Get stdout for writing results
    let mut stdout = io::stdout();

    let mut received_chunks = 0;
    let mut prev_text = String::new();

    tracing::info!("Transcriber ready, waiting for input on stdin...");

    // Main loop: read lines from stdin, process, write to stdout
    while let Some(Ok(line)) = reader.next() {
        // Try to deserialize the line as an IpcAudioChunk
        match serde_json::from_str::<IpcAudioChunk>(&line) {
            Ok(chunk) => {
                received_chunks += 1;

                // Log receipt (not too frequently)
                if received_chunks % 10 == 0 {
                    tracing::debug!(
                        "Received audio chunk: {} samples at {} Hz (total chunks: {})",
                        chunk.samples.len(),
                        chunk.sample_rate,
                        received_chunks
                    );
                }

                // Convert f32 samples to i16 samples for Vosk
                let i16_samples: Vec<i16> = chunk
                    .samples
                    .iter()
                    .map(|&s| (s * 32767.0) as i16)
                    .collect();

                // Process audio with Vosk
                let state = match recognizer.accept_waveform(&i16_samples) {
                    Ok(state) => state,
                    Err(e) => {
                        tracing::error!("Error processing audio chunk: {}", e);
                        continue;
                    }
                };

                let is_final = state == DecodingState::Finalized;

                if is_final {
                    // Extract final result
                    let result = recognizer.final_result();
                    match result.single() {
                        Some(complete_result) => {
                            // Get the text from the result and create a String from it
                            let transcription_text = complete_result.text.to_string();

                            if !transcription_text.is_empty() {
                                let result = IpcTranscriptionResult {
                                    text: transcription_text.clone(),
                                    is_final: true,
                                    confidence: None, // Vosk doesn't provide overall confidence
                                };

                                // Serialize to JSON and send
                                let json = serde_json::to_string(&result)?;
                                writeln!(stdout, "{}", json)?;
                                stdout.flush()?;

                                tracing::info!("Sent final result: {}", transcription_text);
                                prev_text = transcription_text;
                            }
                        }
                        None => {
                            tracing::debug!("Received empty final result");
                        }
                    }
                } else {
                    // Get partial result
                    let partial_result = recognizer.partial_result();
                    // Create a String from the partial result text
                    let transcription_text = partial_result.partial.to_string();

                    if !transcription_text.is_empty() && transcription_text != prev_text {
                        let result = IpcTranscriptionResult {
                            text: transcription_text.clone(),
                            is_final: false,
                            confidence: None,
                        };

                        // Serialize to JSON and send
                        let json = serde_json::to_string(&result)?;
                        writeln!(stdout, "{}", json)?;
                        stdout.flush()?;

                        tracing::debug!("Sent partial result: {}", transcription_text);
                        prev_text = transcription_text;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize input: {}", e);
                tracing::error!("Raw input: {}", line);
            }
        }
    }

    tracing::info!("Input stream ended, shutting down");
    Ok(())
}
