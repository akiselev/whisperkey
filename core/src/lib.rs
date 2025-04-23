use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

pub mod types;

pub use types::{AppOutput, AudioCaptureMsg, CoordinatorMsg};

// Coordinator actor
pub struct Coordinator {
    // Will hold references to other actors
}

// App State for Coordinator
pub struct CoordinatorState {
    // Will hold actor state
}

// Define the Actor implementation for Coordinator
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

        // Send initial status to UI
        ui_sender(AppOutput::UpdateStatus("Initialized".to_string()));

        // Return initial state
        Ok(CoordinatorState {})
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CoordinatorMsg::HandleTest => {
                tracing::info!("Coordinator received HandleTest message");
            }
            CoordinatorMsg::StartListening => {
                tracing::info!("Coordinator: StartListening received");
                // TODO: Forward command to AudioCaptureActor
            }
            CoordinatorMsg::StopListening => {
                tracing::info!("Coordinator: StopListening received");
                // TODO: Forward command to AudioCaptureActor
            }
            CoordinatorMsg::AudioChunk(chunk) => {
                tracing::debug!("Coordinator: Received audio chunk, size: {}", chunk.0.len());
                // TODO: Forward chunk to TranscriptionClientActor (Phase 4)
            }
        }
        Ok(())
    }
}

pub struct CoreHandles {
    pub coordinator: ActorRef<CoordinatorMsg>,
}

pub async fn init_core_actors(
    ui_sender: Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
) -> CoreHandles {
    // Spawn coordinator actor
    let (coordinator, _handle) = Actor::spawn(None, Coordinator {}, ui_sender)
        .await
        .expect("Failed to start coordinator actor");

    CoreHandles { coordinator }
}

// Retain stub for compatibility
pub fn core_hello() {
    tracing::info!("Core library initialized");
}
