use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    audio_capture::AudioCaptureActor,
    transcriber::TranscriberActor,
    types::{AppOutput, AudioCaptureMsg, CoordinatorMsg, TranscriberMsg},
};

pub struct Coordinator {
    // Empty struct, state is in CoordinatorState
}

pub struct CoordinatorState {
    ui_sender: Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
    audio_capture: Option<ActorRef<AudioCaptureMsg>>,
    transcriber: Option<ActorRef<TranscriberMsg>>,
    sample_rate: u32,            // Add sample rate for transcriber
    model_path: Option<PathBuf>, // Path to Vosk model
}

#[ractor::async_trait]
impl Actor for Coordinator {
    type Msg = CoordinatorMsg;
    type State = CoordinatorState;
    type Arguments = (
        Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
        Option<PathBuf>,
    );

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (ui_sender, model_path): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Coordinator actor started");

        // Log model path if provided
        if let Some(path) = &model_path {
            tracing::info!("Using Vosk model at: {:?}", path);
        } else {
            tracing::warn!("No model path provided, using default");
        }

        // Use a fixed sample rate for now - could be configurable later
        let sample_rate = 16000; // 16 kHz is common for speech recognition

        // Spawn the audio capture actor
        let (audio_capture, _) = Actor::spawn(None, AudioCaptureActor {}, myself.clone())
            .await
            .map_err(|e| {
                ActorProcessingErr::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to start audio capture actor: {}", e),
                ))
            })?;

        // Spawn the transcriber actor with model path
        let (transcriber, _) = Actor::spawn(
            None,
            TranscriberActor {},
            (myself.clone(), sample_rate, model_path.clone()),
        )
        .await
        .map_err(|e| {
            ActorProcessingErr::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to start transcriber actor: {}", e),
            ))
        })?;

        // Send initial status to UI
        (ui_sender)(AppOutput::UpdateStatus("Initialized".to_string()));

        // Return initial state
        Ok(CoordinatorState {
            ui_sender,
            audio_capture: Some(audio_capture),
            transcriber: Some(transcriber),
            sample_rate,
            model_path,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CoordinatorMsg::HandleTest => {
                tracing::info!("Coordinator received HandleTest message");
                (state.ui_sender)(AppOutput::UpdateStatus("Test received".to_string()));
            }
            CoordinatorMsg::StartListening => {
                tracing::info!("Coordinator: StartListening received");
                if let Some(audio_capture) = &state.audio_capture {
                    audio_capture.send_message(AudioCaptureMsg::Start)?;
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Starting audio capture...".to_string(),
                    ));
                } else {
                    tracing::error!("Audio capture actor not available");
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Error: Audio capture not available".to_string(),
                    ));
                }
            }
            CoordinatorMsg::StopListening => {
                tracing::info!("Coordinator: StopListening received");
                if let Some(audio_capture) = &state.audio_capture {
                    audio_capture.send_message(AudioCaptureMsg::Stop)?;
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Stopping audio capture...".to_string(),
                    ));
                } else {
                    tracing::error!("Audio capture actor not available");
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Error: Audio capture not available".to_string(),
                    ));
                }
            }
            CoordinatorMsg::AudioChunk(chunk) => {
                // Log less frequently to avoid flooding
                if chunk.0.len() % 1000 == 0 {
                    tracing::debug!("Coordinator: Received audio chunk, size: {}", chunk.0.len());
                }

                // Forward to transcriber if available
                if let Some(transcriber) = &state.transcriber {
                    transcriber.send_message(TranscriberMsg::ProcessAudioChunk(chunk))?;
                }
            }
            CoordinatorMsg::UpdateStatus(status) => {
                // Forward status updates from actors to the UI
                (state.ui_sender)(AppOutput::UpdateStatus(status));
            }
            CoordinatorMsg::TranscriptionResult(transcription) => {
                tracing::info!("Received transcription result: {}", transcription.0);

                // Forward to UI
                (state.ui_sender)(AppOutput::UpdateTranscription(transcription.0));
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Shutdown transcriber if it's running
        if let Some(transcriber) = &state.transcriber {
            let _ = transcriber.send_message(TranscriberMsg::Shutdown);
        }

        tracing::info!("Coordinator stopped");
        Ok(())
    }
}
