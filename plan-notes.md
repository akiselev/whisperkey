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
