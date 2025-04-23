use stakker::{Actor, ActorOwn, Stakker};

pub mod audio_capture;
pub mod types;

use types::CoordinatorMsg;

// Coordinator message enum (REMOVED - now in types.rs)
// pub enum CoordinatorMsg {
//     HandleTest,
// }

// Coordinator actor
pub struct AppCoordinator;

impl AppCoordinator {
    // Add an init function as required by the actor! macro
    pub fn init(_: stakker::CX![]) -> Option<Self> {
        Some(Self)
    }

    // Example handler function (needs to be adapted based on actual usage)
    pub fn handle_test(&mut self, _: stakker::CX![]) {
        tracing::info!("Coordinator received HandleTest message");
    }

    // Add new handlers for Start/Stop Listening
    pub fn handle_start_listening(&mut self, cx: stakker::CX![]) {
        tracing::info!("Coordinator: StartListening received");
        // TODO: Forward command to AudioCaptureActor
    }

    pub fn handle_stop_listening(&mut self, cx: stakker::CX![]) {
        tracing::info!("Coordinator: StopListening received");
        // TODO: Forward command to AudioCaptureActor
    }

    // Add handler for incoming audio chunks
    pub fn handle_internal_audio_chunk(&mut self, cx: stakker::CX![], chunk: types::AudioChunk) {
        tracing::debug!("Coordinator: Received audio chunk, size: {}", chunk.0.len());
        // TODO: Forward chunk to TranscriptionClientActor (Phase 4)
        // TODO: Send status updates to UI
    }
}

pub struct CoreHandles {
    pub coordinator: ActorOwn<AppCoordinator>,
}

pub fn init_core_actors(stakker: &mut Stakker) -> CoreHandles {
    // Use the actor! macro to create the actor instance
    // Assuming no return handler is needed for now (ret_nop!)
    let coordinator = stakker::actor!(
        stakker,
        AppCoordinator::init(),
        stakker::ret_nop!(),
        |cx, msg| {
            match msg {
                CoordinatorMsg::HandleTest => cx.this_mut().handle_test(cx),
                CoordinatorMsg::StartListening => cx.this_mut().handle_start_listening(cx),
                CoordinatorMsg::StopListening => cx.this_mut().handle_stop_listening(cx),
                CoordinatorMsg::InternalAudioChunk(chunk) => {
                    cx.this_mut().handle_internal_audio_chunk(cx, chunk)
                }
            }
        }
    );
    CoreHandles { coordinator }
}

// Retain stub for compatibility
pub fn core_hello() {
    tracing::info!("Core library initialized");
}
