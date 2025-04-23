use std::sync::mpsc;

// Represents a chunk of raw audio data (e.g., f32 samples)
#[derive(Debug, Clone)] // Clone might be useful, Debug for logging
pub struct AudioChunk(pub Vec<f32>);

// Commands for the AudioCaptureActor
#[derive(Debug)]
pub enum AudioCaptureMsg {
    Start,
    Stop,
}

// Messages related to the AppCoordinator
#[derive(Debug)]
pub enum CoordinatorMsg {
    HandleTest, // From Phase 2
    StartListening,
    StopListening,
    AudioChunk(AudioChunk), // Message for coordinator to handle chunks
                            // Add other messages as needed (e.g., for results, state changes)
}

// For UI updates
#[derive(Debug)]
pub enum AppOutput {
    UpdateStatus(String),
}

// Placeholder for transcription results, will be refined later
pub struct FinalTranscription(pub String);
