use serde::{Deserialize, Serialize};

// Represents a chunk of raw audio data (e.g., f32 samples)
#[derive(Debug, Clone)] // Clone might be useful, Debug for logging
pub struct AudioChunk(pub Vec<f32>);

// Commands for the AudioCaptureActor
#[derive(Debug)]
pub enum AudioCaptureMsg {
    Start,
    Stop,
}

// Commands for the AudioProcessorActor
#[derive(Debug)]
pub enum AudioProcessorMsg {
    ProcessChunk(AudioChunk),
    Shutdown,
}

// Commands for the KeyboardOutputActor
#[derive(Debug)]
pub enum KeyboardOutputMsg {
    TypeText(String),
    Enable(bool),
    Shutdown,
}

// Messages related to the AppCoordinator
#[derive(Debug)]
pub enum CoordinatorMsg {
    HandleTest, // From Phase 2
    StartListening,
    StopListening,
    AudioChunk(AudioChunk), // Message for coordinator to handle chunks
    UpdateStatus(String),   // For internal status updates
    TranscriptionResult(FinalTranscription), // From transcriber
    SilenceDetected(bool),  // Silence state change from VAD
    ToggleKeyboardOutput(bool), // Enable/disable keyboard output
}

// For UI updates
#[derive(Debug)]
pub enum AppOutput {
    UpdateStatus(String),
    UpdateTranscription(String),
}

// Placeholder for transcription results, will be refined later
#[derive(Debug, Clone)]
pub struct FinalTranscription(pub String);

// Commands for the TranscriberActor
#[derive(Debug)]
pub enum TranscriberMsg {
    ProcessAudioChunk(AudioChunk),
    Shutdown,
}

// IPC messages (serialized to/from JSON)

// Audio chunk sent to transcriber process via IPC
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcAudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

// Transcription result received from transcriber process via IPC
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcTranscriptionResult {
    pub text: String,
    pub is_final: bool,
    pub confidence: Option<f32>,
}
