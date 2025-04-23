use ractor::{Actor, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;

pub mod audio_capture;
pub mod audio_processor;
pub mod command;
pub mod config;
pub mod coordinator;
pub mod keyboard_output;
pub mod transcriber;
pub mod types;

pub use config::{load_config, save_config, Settings, VadMode};
pub use coordinator::Coordinator;
pub use types::{AppOutput, AudioCaptureMsg, AudioChunk, CoordinatorMsg};

pub struct CoreHandles {
    pub coordinator: ActorRef<CoordinatorMsg>,
}

pub async fn init_core_actors(
    ui_sender: Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
    model_path: Option<PathBuf>,
) -> Result<CoreHandles, Box<dyn std::error::Error>> {
    // Initialize the coordinator actor
    let (coordinator, _handle) = Actor::spawn(None, Coordinator {}, (ui_sender, model_path))
        .await
        .expect("Failed to spawn coordinator actor");

    // Return handles to the actors
    Ok(CoreHandles { coordinator })
}

// Retain stub for compatibility
pub fn core_hello() {
    tracing::info!("Core library initialized");
}
