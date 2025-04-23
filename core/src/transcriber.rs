use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use thiserror::Error;

use crate::types::{
    AudioChunk, CoordinatorMsg, FinalTranscription, IpcAudioChunk, IpcTranscriptionResult,
    TranscriberMsg,
};

#[derive(Error, Debug)]
pub enum TranscriberError {
    #[error("Failed to start transcriber process: {0}")]
    ProcessStartError(String),
    #[error("Failed to communicate with transcriber: {0}")]
    CommunicationError(String),
    #[error("Transcriber process exited unexpectedly")]
    ProcessExitedError,
}

pub struct TranscriberActor {
    // Empty struct as all state is in TranscriberState
}

pub struct TranscriberState {
    // The subprocess
    process: Option<Child>,

    // Communications thread handles
    stdin_thread: Option<JoinHandle<()>>,
    stdout_thread: Option<JoinHandle<()>>,

    // Channel for sending audio chunks to the stdin thread
    chunk_sender: Option<Arc<Mutex<std::sync::mpsc::Sender<AudioChunk>>>>,

    // For sending transcription results back to coordinator
    coordinator: ActorRef<CoordinatorMsg>,

    // Whether we're shutting down
    is_shutting_down: bool,

    // Configuration
    sample_rate: u32,
}

#[ractor::async_trait]
impl Actor for TranscriberActor {
    type Msg = TranscriberMsg;
    type State = TranscriberState;
    type Arguments = (ActorRef<CoordinatorMsg>, u32); // Coordinator ref and sample rate

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (coordinator, sample_rate): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!(
            "TranscriberActor starting with sample rate: {}",
            sample_rate
        );

        // Create a channel for sending audio chunks to the stdin thread
        let (chunk_sender, chunk_receiver) = std::sync::mpsc::channel::<AudioChunk>();
        let chunk_sender = Arc::new(Mutex::new(chunk_sender));

        // Start the transcriber process
        let mut process = Command::new("cargo")
            .args(["run", "--package", "transcriber"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Forward stderr to parent for easy debugging
            .spawn()
            .map_err(|e| {
                ActorProcessingErr::from(TranscriberError::ProcessStartError(e.to_string()))
            })?;

        // Get handles to stdin/stdout
        let stdin = process.stdin.take().ok_or_else(|| {
            ActorProcessingErr::from(TranscriberError::ProcessStartError(
                "Failed to open stdin".to_string(),
            ))
        })?;

        let stdout = process.stdout.take().ok_or_else(|| {
            ActorProcessingErr::from(TranscriberError::ProcessStartError(
                "Failed to open stdout".to_string(),
            ))
        })?;

        let coordinator_clone = coordinator.clone();
        let sample_rate_copy = sample_rate;

        // Start thread for sending audio chunks to transcriber's stdin
        let stdin_thread = thread::spawn(move || {
            let mut stdin = stdin;
            let mut last_error = None;

            // Process audio chunks from the channel
            for chunk in chunk_receiver {
                // Convert to IPC message
                let ipc_chunk = IpcAudioChunk {
                    samples: chunk.0,
                    sample_rate: sample_rate_copy,
                };

                // Serialize to JSON
                match serde_json::to_string(&ipc_chunk) {
                    Ok(json) => {
                        // Send to transcriber's stdin
                        if let Err(e) = writeln!(stdin, "{}", json) {
                            tracing::error!("Failed to write to transcriber stdin: {}", e);
                            last_error = Some(e);
                            break;
                        }

                        // Flush to ensure it gets processed
                        if let Err(e) = stdin.flush() {
                            tracing::error!("Failed to flush transcriber stdin: {}", e);
                            last_error = Some(e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize audio chunk: {}", e);
                        last_error = Some(e.into());
                    }
                }
            }

            if let Some(e) = last_error {
                tracing::error!("Stdin thread exiting due to error: {}", e);
                // Send error back to coordinator
                let _ = coordinator_clone.send_message(CoordinatorMsg::UpdateStatus(format!(
                    "Transcriber communication error: {}",
                    e
                )));
            } else {
                tracing::info!("Stdin thread exiting normally");
            }
        });

        // Start thread for reading transcription results from stdout
        let coordinator_for_stdout = coordinator.clone();
        let stdout_thread = thread::spawn(move || {
            let stdout_reader = BufReader::new(stdout);

            // Read lines from transcriber's stdout
            for line in stdout_reader.lines() {
                match line {
                    Ok(line) => {
                        // Try to deserialize as a transcription result
                        match serde_json::from_str::<IpcTranscriptionResult>(&line) {
                            Ok(result) => {
                                if result.is_final {
                                    tracing::info!("Received transcription: {}", result.text);

                                    // Forward to coordinator
                                    let _ = coordinator_for_stdout.send_message(
                                        CoordinatorMsg::TranscriptionResult(FinalTranscription(
                                            result.text.clone(),
                                        )),
                                    );

                                    // Also send status update
                                    let confidence_str = result
                                        .confidence
                                        .map(|c| format!(" (confidence: {:.1}%)", c * 100.0))
                                        .unwrap_or_default();

                                    let _ = coordinator_for_stdout.send_message(
                                        CoordinatorMsg::UpdateStatus(format!(
                                            "Transcribed: {}{}",
                                            result.text, confidence_str
                                        )),
                                    );
                                } else {
                                    // Partial result - just update status
                                    let _ = coordinator_for_stdout.send_message(
                                        CoordinatorMsg::UpdateStatus(format!(
                                            "Partial: {}",
                                            result.text
                                        )),
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to deserialize transcription result: {}",
                                    e
                                );
                                tracing::error!("Raw line: {}", line);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to read from transcriber stdout: {}", e);

                        // Send error back to coordinator
                        let _ = coordinator_for_stdout.send_message(CoordinatorMsg::UpdateStatus(
                            format!("Transcriber stdout read error: {}", e),
                        ));

                        break;
                    }
                }
            }

            tracing::info!("Stdout thread exiting");

            // Tell coordinator the transcriber has stopped
            let _ = coordinator_for_stdout.send_message(CoordinatorMsg::UpdateStatus(
                "Transcriber process stopped".to_string(),
            ));
        });

        tracing::info!("TranscriberActor started, process and threads running");

        // Tell coordinator the transcriber is ready
        coordinator.send_message(CoordinatorMsg::UpdateStatus(
            "Transcriber ready".to_string(),
        ))?;

        Ok(TranscriberState {
            process: Some(process),
            stdin_thread: Some(stdin_thread),
            stdout_thread: Some(stdout_thread),
            chunk_sender: Some(chunk_sender),
            coordinator,
            is_shutting_down: false,
            sample_rate,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            TranscriberMsg::ProcessAudioChunk(chunk) => {
                // Don't process chunks during shutdown
                if state.is_shutting_down {
                    return Ok(());
                }

                // Check if we have a valid sender
                if let Some(sender) = &state.chunk_sender {
                    // Try to send the chunk to the stdin thread
                    match sender.lock() {
                        Ok(sender) => {
                            if let Err(e) = sender.send(chunk) {
                                tracing::error!(
                                    "Failed to send audio chunk to stdin thread: {}",
                                    e
                                );
                                state
                                    .coordinator
                                    .send_message(CoordinatorMsg::UpdateStatus(format!(
                                        "Transcriber error: {}",
                                        e
                                    )))?;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to lock chunk sender: {}", e);
                            state
                                .coordinator
                                .send_message(CoordinatorMsg::UpdateStatus(format!(
                                    "Transcriber error: Failed to lock chunk sender: {}",
                                    e
                                )))?;
                        }
                    }
                } else {
                    tracing::error!("No chunk sender available");
                }
            }
            TranscriberMsg::Shutdown => {
                tracing::info!("Shutting down transcriber...");
                state.is_shutting_down = true;

                // Drop the chunk sender to stop the stdin thread
                state.chunk_sender = None;

                // Try to kill the process
                if let Some(mut process) = state.process.take() {
                    match process.kill() {
                        Ok(_) => tracing::info!("Killed transcriber process"),
                        Err(e) => tracing::error!("Failed to kill transcriber process: {}", e),
                    }
                }

                // Don't wait for the threads - they'll exit when the process ends

                state
                    .coordinator
                    .send_message(CoordinatorMsg::UpdateStatus(
                        "Transcriber shut down".to_string(),
                    ))?;
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Make sure we shutdown properly
        state.is_shutting_down = true;

        // Drop the chunk sender to stop the stdin thread
        state.chunk_sender = None;

        // Kill the process if it's still running
        if let Some(mut process) = state.process.take() {
            match process.kill() {
                Ok(_) => tracing::info!("Killed transcriber process during shutdown"),
                Err(e) => {
                    tracing::error!("Failed to kill transcriber process during shutdown: {}", e)
                }
            }
        }

        // Don't wait for threads - they'll exit when the process ends

        tracing::info!("TranscriberActor stopped");
        Ok(())
    }
}
