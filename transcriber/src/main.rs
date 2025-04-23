use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

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

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    tracing::info!("Transcriber stub started");

    // Get stdin as a buffered reader
    let stdin = io::stdin();
    let mut reader = stdin.lock().lines();

    // Get stdout for writing results
    let mut stdout = io::stdout();

    // Simple counter for demo
    let mut received_chunks = 0;

    tracing::info!("Waiting for input on stdin...");

    // Main loop: read lines from stdin, process, write to stdout
    while let Some(Ok(line)) = reader.next() {
        // Try to deserialize the line as an IpcAudioChunk
        match serde_json::from_str::<IpcAudioChunk>(&line) {
            Ok(chunk) => {
                received_chunks += 1;

                // Log receipt (not too frequently)
                if received_chunks % 10 == 0 {
                    tracing::info!(
                        "Received audio chunk: {} samples at {} Hz (total chunks: {})",
                        chunk.samples.len(),
                        chunk.sample_rate,
                        received_chunks
                    );
                }

                // Generate a dummy transcription result every 50 chunks
                if received_chunks % 50 == 0 {
                    let result = IpcTranscriptionResult {
                        text: format!("Stub transcription #{}", received_chunks / 50),
                        is_final: true,
                        confidence: Some(0.95),
                    };

                    // Serialize to JSON
                    let json = serde_json::to_string(&result).unwrap();

                    // Write to stdout
                    writeln!(stdout, "{}", json).unwrap();
                    stdout.flush().unwrap();

                    tracing::info!("Sent result: {}", result.text);
                }
            }
            Err(e) => {
                tracing::error!("Failed to deserialize input: {}", e);
                tracing::error!("Raw input: {}", line);
            }
        }
    }

    tracing::info!("Input stream ended, shutting down");
}
