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

## 1. Dependencies (`core/`)

- Added `cpal` crate for audio I/O (already in workspace dependencies).
- Using `std::sync::mpsc` for audio chunk communication between capture thread and coordinator.

## 2. Audio Types (`core/src/types.rs`)

- Created file.
- Defined `AudioChunk(Vec<f32>)`.
- Defined `AudioCaptureCmd { Start, Stop }`.
- Added placeholder `AppOutput { UpdateStatus(String) }` for UI communication back from core.

## 3. Audio Capture Actor (`core/src/audio_capture.rs`)

- Created file and implemented `AudioCaptureActor`.
- `init` takes `mpsc::Sender<AudioChunk>`.
- `handle` processes `AudioCaptureCmd::Start` and `AudioCaptureCmd::Stop`.
- `start_capture` uses `cpal` to get the default input device and stream.
- `cpal` stream callback converts data to `f32`, wraps in `AudioChunk`, and sends via `mpsc::Sender`.
- The `cpal::Stream` object is stored in the actor state to keep the stream alive.
- `stop_capture` drops the `Stream` object.
- Added basic error handling using `thiserror` and logging.

## 4. Coordinator Integration (`core/src/coordinator.rs`)

- Created file `core/src/coordinator.rs` (was missing).
- Added `Actor<AudioCaptureActor>` handle to `AppCoordinator`.
- Modified `CoordinatorMsg` to include `StartListening`, `StopListening`, `InternalAudioChunk(AudioChunk)`.
- Modified `AppCoordinator::init`:
  - Accepts a generic `ui_sender: Fn(AppOutput) + Send + Sync + 'static` (implemented via `Box<dyn Fn(...)>`).
  - Stores `ui_sender`.
  - Creates `mpsc::channel::<AudioChunk>()`.
  - Spawns `AudioCaptureActor`, passing the `mpsc::Sender`.
  - Spawns a dedicated thread to receive from `mpsc::Receiver<AudioChunk>`.
  - Receiver thread loop sends `CoordinatorMsg::InternalAudioChunk` to the coordinator using `self_ref.defer()`. Requires getting `actor_ref()` in `init`.
- Implemented `handle` for new messages:
  - `StartListening`/`StopListening`: Sends `AudioCaptureCmd::Start`/`Stop` to `audio_capture_actor` using `send()`.
  - `InternalAudioChunk`: Logs receipt (size).
- Implemented `send_status_to_ui` helper method using the stored `ui_sender` closure.
- Sends status updates to UI on start/stop and init.
- Modified `core/src/lib.rs`:
  - Added `pub mod audio_capture`.
  - Updated `init_core_actors` to be generic and accept `ui_sender`, passing it to `AppCoordinator::init`.

## 5. UI Integration (`src/main.rs`)

- Added `flume` dependency to root `Cargo.toml` (needed for `run_with_receive`).
- Defined `AppInput::{StartListening, StopListening}`.
- `AppModel::Output` is now `whisperkey_core::types::AppOutput`.
- Added `status_text: String` to `AppModel`.
- Updated `view!` macro:
  - Added `gtk::Label` bound to `model.status_text`.
  - Added "Start Listening" and "Stop Listening" `gtk::Button`s triggering corresponding `AppInput` messages.
- Updated `AppModel::init` to take `CoreHandles` as init parameter.
- Updated `AppModel::update`:
  - Handles `AppInput::StartListening`/`StopListening`: Sends `CoordinatorMsg::StartListening`/`StopListening` to coordinator using `core_handles.coordinator.defer()`.
- Implemented `AppModel::output` method:
  - Handles `AppOutput::UpdateStatus`: Updates `model.status_text`.
- Updated `main` function:
  - Creates a `flume` channel.
  - Creates a closure `ui_sender_closure` capturing the `flume` sender to pass to `init_core_actors`.
  - Calls `init_core_actors` _before_ running the app, passing the closure.
  - Uses `app.run_with_receive(core_handles, receiver)` to integrate the flume receiver with the Relm4 event loop.

## 6. Notes

- The audio capture pipeline (Capture Actor -> mpsc -> Receiver Thread -> Coordinator Actor) is established.
- UI can now trigger start/stop of audio capture.
- Core coordinator can send status updates back to the UI label.
- Next: Phase 4 - Stub Transcriber Process & IPC.
