use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use thiserror::Error;

use crate::types::{AudioCaptureMsg, AudioChunk, CoordinatorMsg};

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error("Failed to initialize audio input: {0}")]
    InitError(String),
    #[error("Failed to build audio stream: {0}")]
    StreamError(String),
}

// This will be our thread-local context that manages the CPAL stream
struct StreamContext {
    _stream: cpal::Stream, // The underscore prevents "unused" warnings
}

impl Drop for StreamContext {
    fn drop(&mut self) {
        // The stream is automatically stopped when dropped
        tracing::info!("Audio stream dropped");
    }
}

// We'll use this to track if audio is active without storing the actual stream
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioState {
    Started,
    Stopped,
}

pub struct AudioCaptureActor {
    // Empty struct, all state is in AudioCaptureState
}

pub struct AudioCaptureState {
    audio_state: AudioState,
    coordinator: ActorRef<CoordinatorMsg>,
    // We'll store the stream context in a thread-local variable
    // and just track the state here
}

thread_local! {
    static STREAM_CONTEXT: std::cell::RefCell<Option<StreamContext>> = std::cell::RefCell::new(None);
}

#[ractor::async_trait]
impl Actor for AudioCaptureActor {
    type Msg = AudioCaptureMsg;
    type State = AudioCaptureState;
    type Arguments = ActorRef<CoordinatorMsg>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        coordinator: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("AudioCaptureActor started");

        Ok(AudioCaptureState {
            audio_state: AudioState::Stopped,
            coordinator,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            AudioCaptureMsg::Start => {
                if state.audio_state == AudioState::Stopped {
                    match self.start_capture(state.coordinator.clone()) {
                        Ok(()) => {
                            state.audio_state = AudioState::Started;
                            tracing::info!("Audio capture started");
                            // Send status update back to coordinator
                            state
                                .coordinator
                                .send_message(CoordinatorMsg::UpdateStatus(
                                    "Audio capture started".to_string(),
                                ))?;
                        }
                        Err(e) => {
                            tracing::error!("Failed to start audio capture: {}", e);
                            state
                                .coordinator
                                .send_message(CoordinatorMsg::UpdateStatus(format!(
                                    "Error: {}",
                                    e
                                )))?;
                        }
                    }
                } else {
                    tracing::info!("Audio capture already started");
                }
            }
            AudioCaptureMsg::Stop => {
                if state.audio_state == AudioState::Started {
                    // Stop the stream by dropping it
                    STREAM_CONTEXT.with(|context| {
                        *context.borrow_mut() = None;
                    });

                    state.audio_state = AudioState::Stopped;
                    tracing::info!("Audio capture stopped");

                    // Send status update back to coordinator
                    state
                        .coordinator
                        .send_message(CoordinatorMsg::UpdateStatus(
                            "Audio capture stopped".to_string(),
                        ))?;
                } else {
                    tracing::info!("Audio capture already stopped");
                }
            }
        }
        Ok(())
    }
}

impl AudioCaptureActor {
    fn start_capture(
        &self,
        coordinator: ActorRef<CoordinatorMsg>,
    ) -> Result<(), AudioCaptureError> {
        // Get default host
        let host = cpal::default_host();

        // Get default input device
        let device = host
            .default_input_device()
            .ok_or_else(|| AudioCaptureError::InitError("No input device found".to_string()))?;

        tracing::info!(
            "Using input device: {}",
            device.name().unwrap_or_else(|_| "Unknown".to_string())
        );

        // Get supported config
        let config = device
            .default_input_config()
            .map_err(|e| AudioCaptureError::InitError(format!("Default config error: {}", e)))?;

        tracing::info!("Using input config: {:?}", config);

        // Clone the coordinator ref for the closure
        let coordinator_ref = coordinator.clone();

        // Build the stream
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Create a copy of the data
                    let chunk = AudioChunk(data.to_vec());

                    // Send to coordinator
                    if let Err(e) = coordinator_ref.send_message(CoordinatorMsg::AudioChunk(chunk))
                    {
                        tracing::error!("Failed to send audio chunk: {}", e);
                    }
                },
                move |err| {
                    tracing::error!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioCaptureError::StreamError(format!("Stream error: {}", e)))?;

        // Start the stream
        stream
            .play()
            .map_err(|e| AudioCaptureError::StreamError(format!("Failed to play stream: {}", e)))?;

        // Store the stream in our thread-local
        STREAM_CONTEXT.with(|context| {
            *context.borrow_mut() = Some(StreamContext { _stream: stream });
        });

        Ok(())
    }
}
