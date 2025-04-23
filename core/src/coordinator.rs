use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

use crate::{
    audio_capture::AudioCaptureActor,
    types::{AppOutput, AudioCaptureMsg, CoordinatorMsg},
};

pub struct Coordinator {
    // Empty struct, state is in CoordinatorState
}

pub struct CoordinatorState {
    ui_sender: Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
    audio_capture: Option<ActorRef<AudioCaptureMsg>>,
}

#[ractor::async_trait]
impl Actor for Coordinator {
    type Msg = CoordinatorMsg;
    type State = CoordinatorState;
    type Arguments = Arc<dyn Fn(AppOutput) + Send + Sync + 'static>;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        ui_sender: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Coordinator actor started");

        // Spawn the audio capture actor
        let (audio_capture, _) = Actor::spawn(None, AudioCaptureActor {}, myself.clone())
            .await
            .map_err(|e| {
                ActorProcessingErr::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to start audio capture actor: {}", e),
                ))
            })?;

        // Send initial status to UI
        (ui_sender)(AppOutput::UpdateStatus("Initialized".to_string()));

        // Return initial state
        Ok(CoordinatorState {
            ui_sender,
            audio_capture: Some(audio_capture),
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

                // In Phase 4, we'll forward this to the transcriber process
                // For now, we just acknowledge receipt
            }
            CoordinatorMsg::UpdateStatus(status) => {
                // Forward status updates from actors to the UI
                (state.ui_sender)(AppOutput::UpdateStatus(status));
            }
        }
        Ok(())
    }
}
