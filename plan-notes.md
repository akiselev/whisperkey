# Phase 1 Implementation Notes (Project Setup & Basic UI Shell)

## 1. Workspace Setup

- Updated `Cargo.toml` to include all required workspace members: `.`, `core`, and `transcriber`.
- Confirmed: `core` crate already existed. `transcriber` was missing and has been created.

## 2. Root Crate (`src/`) Setup

- `Cargo.toml` dependencies are present, but `relm4` and `gtk` are commented out. (Will need to enable/add these for actual GTK/Relm4 UI work.)
- `src/main.rs` exists; has substantial logic, but not a minimal Relm4/GTK app as described in the plan. (May need to refactor to match plan if strict adherence is required.)

## 3. Core Crate (`core/`) Setup

- `core/Cargo.toml` exists with dependencies. Confirmed as a library crate.
- Added a placeholder function `pub fn core_hello()` that logs a message using `tracing` per plan.md.

## 4. Transcriber Crate (`transcriber/`) Setup

- Created `transcriber/Cargo.toml` with dependencies: `tracing`, `tracing-subscriber`.
- Created `transcriber/src/main.rs` stub: initializes tracing, logs a startup message, and sleeps.

## 5. Basic System Tray Stub

- Created `src/tray.rs` as a stub with a placeholder `init_tray()` function.

# Phase 2 Implementation Notes (Ractor & Core Initialization)

## 1. Ractor Setup (`core/`)

- Updated `core/Cargo.toml` to replace Stakker with Ractor v0.13 dependency.
- Created a proper Actor implementation for `Coordinator` with the `Actor` trait.
- Defined messages in `types.rs`:
  - `CoordinatorMsg::HandleTest` for basic testing
  - `CoordinatorMsg::StartListening` and `StopListening` for future audio capture
  - `CoordinatorMsg::AudioChunk` for receiving audio data
- Added `AppOutput` enum for sending messages back to the UI.
- Created `CoreHandles` struct holding `ActorRef<CoordinatorMsg>`.
- Implemented `init_core_actors` function that returns `CoreHandles`.

## 2. Ractor Integration (`src/`)

- Added `tokio` dependency with full features for async runtime.
- Added `flume` dependency for message passing between UI thread and async tasks.
- Updated `main.rs` to use `#[tokio::main]` for the async runtime.
- Set up bidirectional communication between UI and core:
  - Core-to-UI: Using tokio channel and a closure passed to `init_core_actors`.
  - UI-to-Core: Using `actor_ref.send_message()` from button handlers.

## 3. UI→Core Communication

- Defined proper Relm4 components:
  - `AppInput` enum with messages for UI events
  - `AppModel` struct with state and core handles
  - View with buttons for testing core functionality
- Implemented basic message flow:
  - Button clicks generate `AppInput` messages
  - `AppModel::update` handles messages and forwards to core via `send_message()`
  - Core responds via the tokio channel
  - Updates are processed in the UI via `AppInput::ProcessOutput`

## 4. Improvements

- Used Ractor's async trait implementation for the `Actor` trait.
- Set up proper bidirectional communication channels between the UI and core.
- Added status text UI element to display updates from core.
- Implemented proper error handling for message passing.
- Used tokio tasks for background processing.

## 5. Notes on Ractor vs Stakker

- Ractor follows an Erlang-inspired actor model with explicit message passing.
- Major differences from Stakker:
  - Ractor uses async/await for message handling.
  - Messages are explicitly passed with `send_message()` rather than using macros.
  - Actor state is managed through the `pre_start` and `handle` methods.
  - Supervision is built into Ractor for handling actor failures.

# Phase 3 Implementation Notes (Audio Capture)

## 1. Audio Types and Modules

- Updated `core/src/types.rs` to include `AudioChunk(Vec<f32>)` for storing audio data.
- Added `AudioCaptureMsg { Start, Stop }` for controlling the audio capture actor.
- Added `CoordinatorMsg::UpdateStatus(String)` for internal status updates between actors.
- Created new modules:
  - `core/src/audio_capture.rs` for the audio capture actor
  - `core/src/coordinator.rs` for the coordinator implementation

## 2. Audio Capture Actor Implementation

- Created `AudioCaptureActor` with proper Ractor `Actor` trait implementation.
- Added audio capture error handling using `thiserror` crate.
- `start_capture` method initializes CPAL audio inputs:
  - Gets default host and input device
  - Configures the device using default input config
  - Builds input stream with callback that creates `AudioChunk` instances
  - Sends chunks to the coordinator actor
- Implemented message handling for `Start`/`Stop` commands:
  - `Start`: Initializes and stores the CPAL stream
  - `Stop`: Drops the stream to stop capture
  - Both send status updates back to coordinator

## 3. Coordinator Implementation

- Moved coordinator from `lib.rs` to dedicated `coordinator.rs` file.
- Created `CoordinatorState` with fields:
  - `ui_sender`: Function to send updates to UI
  - `audio_capture`: Reference to audio capture actor
- `pre_start` now spawns the audio capture actor and passes itself as argument.
- Updated message handling:
  - `StartListening`: Forwards to audio capture actor
  - `StopListening`: Forwards to audio capture actor
  - `AudioChunk`: Logs receipt (will be forwarded to transcriber in Phase 4)
  - `UpdateStatus`: Forwards status messages from actors to UI

## 4. Integration

- Updated `core/src/lib.rs` to include new modules and use the new coordinator implementation.
- Removed the old coordinator implementation from `lib.rs`.
- Kept the existing UI integration which already had:
  - `StartListening`/`StopListening` buttons
  - Status label for updates
  - Message passing from UI to core and back

## 5. Error Handling

- Added proper error types for audio capture issues.
- Used Rust's `Result` type throughout for robust error handling.
- Added appropriate logging at different levels (info, debug, error).
- Status updates provide feedback to the user via the UI.

## 6. Notes on Audio Capture

- Audio is captured using CPAL's default input device and configuration.
- Audio data is sent as raw f32 samples in `AudioChunk` messages.
- The stream is properly cleaned up when stopped.
- In Phase 4, audio chunks will be forwarded to the transcriber process.

# Phase 4 Implementation Notes (Stub Transcriber Process & IPC Client)

## 1. IPC Types and Dependencies

- Added `serde` and `serde_json` dependencies to both `core` and `transcriber` crates.
- Defined IPC types in `core/src/types.rs`:
  - `IpcAudioChunk` with `samples` and `sample_rate` fields
  - `IpcTranscriptionResult` with `text`, `is_final`, and `confidence` fields
- Added message type `TranscriberMsg` with variants for processing audio and shutdown
- Added `CoordinatorMsg::TranscriptionResult` for passing results back to the UI
- Added `AppOutput::UpdateTranscription` for displaying results in the UI

## 2. Transcriber Stub Implementation

- Implemented `transcriber/src/main.rs` with basic JSON IPC:
  - Reads JSON lines from stdin, parses as `IpcAudioChunk`
  - Generates dummy transcription results (`Stub transcription #N`) every 50 chunks
  - Serializes results as JSON and writes to stdout
  - Proper error handling and logging

## 3. Transcriber Actor Implementation

- Created `core/src/transcriber.rs` with the `TranscriberActor` implementation:
  - Spawns the transcriber process with `cargo run --package transcriber`
  - Creates separate threads for stdin (sending chunks) and stdout (receiving results)
  - Converts `AudioChunk` to `IpcAudioChunk` with proper sample rate
  - Parses transcription results and forwards them to the coordinator
  - Proper error handling, shutdown logic, and cleanup

## 4. Coordinator Integration

- Updated `core/src/coordinator.rs` to include the transcriber:
  - Added `transcriber` field to `CoordinatorState`
  - Spawns the transcriber actor in `pre_start`
  - Forwards audio chunks to the transcriber
  - Handles transcription results and forwards them to the UI
  - Added proper cleanup in `post_stop`

## 5. UI Integration

- Enhanced the UI in `src/main.rs`:
  - Added a `transcription_text` field to store the current transcription
  - Added a scrollable text view to display transcription results
  - Improved the layout with horizontal button arrangement
  - Added handler for `AppOutput::UpdateTranscription`
  - Used thread-local storage to communicate with the text view

## 6. Error Handling

- Added proper error types and handling throughout the pipeline
- Used `thiserror` for error definitions
- Added status updates for error conditions
- Graceful handling of process and communication failures

## 7. Notes

- The IPC pipeline is now fully implemented:
  - Audio capture → Coordinator → Transcriber Actor → Transcriber Process → Results → UI
- The transcriber process runs as a separate executable, communicating via stdin/stdout
- JSON is used for IPC serialization
- The UI now shows both status updates and transcription results
- Dummy/stub transcription will be replaced with real Vosk integration in Phase 5

# Phase 5 Implementation Notes (Vosk Integration)

## 1. Vosk Dependencies

- Added `whisper-rs` dependency to `core/Cargo.toml` using the workspace definition
- Added `vosk` library dependency to `transcriber/Cargo.toml` for the separate process
- Used clap for command-line argument parsing in the transcriber process

## 2. Transcriber Process Implementation

- Replaced the stub transcriber with real Vosk-based transcription in `transcriber/src/main.rs`:
  - Added command-line arguments for model path and sample rate using clap
  - Initialized Vosk model and recognizer with proper options
  - Process audio chunks from stdin and feed samples to Vosk
  - Get both partial and final results from Vosk and forward them via IPC
  - Added proper confidence reporting when available
  - Implemented proper error handling for model loading and recognition

## 3. Transcriber Actor Enhancement

- Updated `core/src/transcriber.rs` to support Vosk integration:
  - Added model path handling to the actor and its messages
  - Pass model path to transcriber process as command-line argument
  - Enhanced error handling for process startup and communication

## 4. Coordinator Enhancement

- Updated `core/src/coordinator.rs` to handle model path:
  - Added model path field to store and pass to transcriber
  - Added model path to actor arguments
  - Log model path information

## 5. UI Enhancement

- Updated `src/main.rs` to support model path selection:
  - Added a default model path lookup function for testing
  - Pass model path to core during initialization
  - Improved transcription display in the UI

## 6. Notes

- Vosk integration is now functional:
  - Audio is captured via CPAL
  - Audio chunks are sent to the transcriber process
  - Vosk processes the audio and generates transcription results
  - Results are sent back via IPC to the UI
- The current implementation requires a model path to be provided
- Future phases will add configuration of model paths

# Phase 6 Implementation Notes (Configuration & E2E Flow)

## 1. Configuration System Implementation

- Added `config` and `dirs` dependencies to `core/Cargo.toml` for configuration management
- Added `toml` dependency for serialization of config files
- Created `core/src/config.rs` with:
  - `Settings` struct with `model_path` field
  - Functions for loading/saving configuration from/to file
  - Default settings when no config file exists
  - Configuration directory management using `dirs` crate
  - Proper error handling for config operations

## 2. Settings UI Implementation

- Created `src/settings.rs` with a GTK4 settings dialog:
  - Dialog to view and edit the model path
  - File chooser for selecting Vosk model directory
  - Save/cancel buttons with proper responses
  - Integration with configuration system

## 3. Coordinator Integration

- Updated `coordinator.rs` to use the configuration system:
  - Added `config` field to store loaded settings
  - Modified initialization to load config and use model path from it
  - Added precedence logic: CLI model path overrides config

## 4. UI Integration

- Updated `main.rs` to include settings dialog:
  - Added menu button with Settings option
  - Added settings dialog launcher
  - Updated initialization to use config for model path
  - Added status messaging when settings are updated

## 5. Configuration Storage

- Config is stored in standard OS-specific locations:
  - Windows: %APPDATA%\whisperkey
  - macOS: ~/Library/Application Support/whisperkey
  - Linux: ~/.config/whisperkey
- Configuration uses TOML format for human readability

## 6. Notes

- The end-to-end flow is now complete:
  1. Configuration is loaded at startup
  2. Model path is determined from config or CLI arguments
  3. Audio is captured when listening is started
  4. Audio is processed by the Vosk transcriber
  5. Transcription results are displayed in the UI
  6. Settings can be updated via the settings dialog
- The application now handles the complete workflow from audio capture to transcription display
- Configuration persistence allows for easy setup across sessions

# Phase 7 Implementation Notes (Audio Pre-processing)

## 1. Audio Processing Dependencies

- Added `nnnoiseless` for noise reduction and `webrtc-vad` for voice activity detection to core/Cargo.toml.
- Nnnoiseless is used for real-time noise reduction and cleanup of audio signals.
- WebRTC VAD is used for detecting speech in the audio stream and determining when silence occurs.

## 2. Audio Processor Actor Implementation

- Created the `AudioProcessorActor` to handle audio pre-processing tasks, implemented with Ractor for actor model concurrency.
- Major components:
  - Noise reduction using the DenoiseState from nnnoiseless.
  - Voice activity detection using WebRTC's VAD, running in its own thread due to Send/Sync limitations.
  - Silence detection with configurable thresholds, sending notifications to the coordinator when silence is detected.

## 3. Thread-Based VAD Implementation

- The WebRTC VAD implementation is not Send + Sync, so it was placed in a dedicated thread.
- Implemented a message-passing system between the actor and the VAD thread:
  - `VadRequest` enum for sending audio chunks and commands to the VAD thread.
  - `VadResponse` enum for receiving detection results or errors.
  - The VAD thread processes audio samples and returns a boolean indicating voice presence.

## 4. Configuration System Enhancement

- Enhanced the `Settings` struct in config.rs with:
  - `enable_denoise`: Boolean to enable/disable noise reduction.
  - `enable_vad`: Boolean to enable/disable voice activity detection.
  - `vad_mode`: Enum for WebRTC's VAD sensitivity modes (Quality, LowBitrate, Aggressive, VeryAggressive).
  - `vad_energy_threshold`: Float for backup VAD using simple energy-based detection.
  - `silence_threshold_ms`: Duration in milliseconds to consider as silence.
- These settings are persisted in the TOML configuration file and can be adjusted via the settings dialog.

## 5. UI Integration

- Enhanced the settings dialog to include audio processing options:
  - Checkbox for enabling/disabling noise reduction.
  - Checkbox for enabling/disabling voice activity detection.
  - ComboBox for selecting VAD mode.
  - SpinButton for configuring energy threshold.
  - SpinButton for configuring silence threshold duration.
- Settings are properly saved and loaded from the configuration file.

## 6. Pipeline Integration

- Integrated the audio processor into the existing pipeline:
  - Changed the data flow to: `AudioCapture → AudioProcessor → Transcriber → Coordinator → UI`.
  - The audio processor sits between audio capture and transcription, processing all audio chunks.
  - Processed chunks are sent to the transcriber for speech recognition.
  - VAD status (speech/silence) is sent directly to the coordinator.

## 7. Fallback VAD Mechanism

- Implemented a simple energy-based VAD as a fallback mechanism:
  - Calculate the energy of audio chunks (sum of squared samples).
  - Compare the energy level to a configurable threshold.
  - Useful when WebRTC VAD is not available or disabled.

## 8. Notes on Implementation Challenges

- Encountered issues with the WebRTC VAD library's thread safety, requiring a dedicated thread approach.
- Used message-passing to communicate between the actor model and the VAD thread.
- Learned how nnnoiseless DenoiseState's API works for efficient audio processing.
- Leveraged existing configuration and settings systems to make preprocessing features configurable.

## 9. Future Improvements

- Potential for more specialized audio filters (bandpass, high-pass, etc.).
- Option to revert to WebRTC's full audio processing module for more advanced features.
- Ability to save and load audio processing presets for different environments.
- Visual indicators in the UI to show VAD status and audio energy levels.
