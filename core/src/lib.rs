use ractor::{Actor, ActorRef};
use std::sync::Arc;

pub mod audio_capture;
pub mod coordinator;
pub mod types;

pub use coordinator::Coordinator;
pub use types::{AppOutput, AudioCaptureMsg, CoordinatorMsg};

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
