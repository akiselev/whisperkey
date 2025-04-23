**Assumptions:**

1.  **UI Framework:** **GTK4-rs / Relm4** for the main application window and UI elements (Root Crate: `src/`).
2.  **Core Logic:** **Stakker** actors, audio pipeline, state management, command handling, IPC client, activation logic, configuration handling resides in the **`core/`** library crate.
3.  **System Tray:** **`tauri-plugin-system-tray`** integrated in the Root Crate (`src/`), interacting with the `core` crate.
4.  **Actor Framework:** **Stakker** used within the `core/` crate.
5.  **Concurrency:** Standard **multi-threading** (`std::thread`, `std::sync::mpsc`) used within the `core/` crate.
6.  **Transcription Backend:** **Vosk-rs** running in a separate process (`transcriber/` crate).
7.  **Transcription Process IPC:** stdin/stdout using **JSON Lines** initially.
8.  **Wake Word Engine:** A thread-compatible library (e.g., `porcupine-rs`) integrated in `core/`.
9.  **Global Activation:** A thread-compatible hotkey library (e.g., `inputbot`) integrated in `core/`.
10. **Output:** `enigo` used within `core/`.
11. **Configuration:** `config-rs` with TOML format, logic primarily in `core/`, settings UI in root crate (`src/`).
12. **Logging:** `tracing` with file and console output.
13. **Target Platforms:** macOS, Windows, Linux.
14. **Project Structure:**
    - `whisperkey/`
      - `Cargo.toml`: Workspace (`members = [".", "core", "transcriber"]`). Root depends on `core`.
      - `src/`: Main binary - GTK/Relm4 UI, system tray, `main`.
      - `core/`: Library - Stakker actors, logic, audio, state, config.
      - `transcriber/`: Binary - Vosk process.

---

**Phase 1: Project Setup & Basic UI Shell**

- **Goal:** Initialize crates, dependencies, basic GTK window, and system tray icon. No actors or audio yet.
- **Tasks:**
  1.  **Workspace Setup:** Create `whisperkey/Cargo.toml` defining the workspace.
  2.  **Root Crate (`src/`) Setup:** Add deps (`gtk4`, `relm4`, `tracing`, `tracing-subscriber`, `tauri-plugin-system-tray`, `whisperkey-core`). Basic `main.rs` with GTK/Relm4 structure, empty `AppModel`, `AppWidgets`, empty `AppInput`/`AppOutput`. Run Relm4 app.
  3.  **Core Crate (`core/`) Setup:** `[lib]`. Add deps (`stakker`, `tracing`). Placeholder function `core_hello()`.
  4.  **Transcriber Crate (`transcriber/`) Setup:** Add deps (`tracing`, `tracing-subscriber`). `main.rs` logs start message.
  5.  **Basic System Tray (`src/tray.rs`):** Use `tauri-plugin-system-tray` for icon and basic Quit menu item. Handle threading.
  6.  **Documentation:** `README.md`: Structure, deps, build command.
  7.  **Testing:** Manual: `cargo run`. Verify empty GTK window, tray icon, Quit works. Logs show init messages. Run transcriber stub.

---

**Phase 2: Stakker & Core Initialization**

- **Goal:** Introduce Stakker, define minimal actors in `core`, initialize them from `src`, establish UI -> Core message passing.
- **Tasks:**
  1.  **Stakker Setup (`core/src/lib.rs`):** Define `AppCoordinator` actor placeholder. Define `init_core_actors` function returning `CoreHandles { coordinator: Actor<AppCoordinator> }`.
  2.  **Stakker Integration (`src/main.rs`):** Create `Stakker` instance. Call `core::init_core_actors`. Store `CoreHandles`. Ensure Stakker runs (e.g., driven by GTK idle loop).
  3.  **Basic UI->Core Communication (`src/ui.rs`, `core/src/lib.rs`):**
      - Add "Test Core" GTK button (`src/`).
      - Define `AppInput::TestCore` (`src/`).
      - Define `CoordinatorMsg::HandleTest` (`core/`).
      - Button click handler (`src/`) sends `AppInput::TestCore`.
      - `AppModel::update` (`src/`) handles `AppInput::TestCore`: use `core_handles.coordinator.defer_msg(CoordinatorMsg::HandleTest);` (using `defer_msg` as it originates from UI thread).
      - `AppCoordinator::handle_msg` (`core/`) handles `CoordinatorMsg::HandleTest`: logs message.
  4.  **Documentation:** Explain Stakker initialization, UI-to-Core message flow using `defer_msg`.
  5.  **Testing:** Manual: `cargo run`. Click "Test Core". Verify coordinator log message appears.

---

**Phase 3: Audio Capture Implementation**

- **Goal:** Implement `AudioCaptureActor` in `core` using `cpal` and `std::thread`, send raw audio chunks via `mpsc`.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add `cpal`.
  2.  **Audio Types (`core/src/types.rs`):** `AudioChunk(Vec<f32>)`, `AudioCaptureCmd { Start, Stop }`.
  3.  **Audio Capture Actor (`core/src/audio_capture.rs`):** Implement `Actor`. `init` takes `mpsc::Sender<AudioChunk>`. `handle_msg` handles `AudioCaptureCmd::Start`/`Stop`. `Start` spawns `std::thread` for `cpal` stream callback. Callback converts samples, sends via `mpsc::Sender`.
  4.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - `AppCoordinatorActor`:
        - `init`: Create `mpsc::channel::<AudioChunk>()`. Spawn `AudioCaptureActor`, pass sender. Store `Actor<AudioCaptureActor>` handle. Spawn thread to receive from `mpsc::Receiver<AudioChunk>` and `self.defer_msg(InternalAudioChunk(chunk))` (using `defer_msg` from receiver thread to actor).
        - Define `CoordinatorMsg { StartListening, StopListening, InternalAudioChunk(AudioChunk) }`.
        - Handle `StartListening`/`StopListening`: `self.audio_capture_actor.send_msg(AudioCaptureCmd::Start)` (using `send_msg` as it's actor-to-actor within handler).
        - Handle `InternalAudioChunk`: Log arrival.
  5.  **UI Integration (`src/ui.rs`):**
      - Add "Start/Stop Listening" buttons, Status label.
      - Define `AppInput { StartListening, StopListening }`, `AppOutput { UpdateStatus(String) }`.
      - Button clicks send `AppInput`.
      - `AppModel::update` handles inputs: `core_handles.coordinator.defer_msg(CoordinatorMsg::StartListening)`.
      - Coordinator (`core`) sends status updates back to UI (e.g., via another channel or Relm4 sender passed during init). `AppModel::update` handles `AppOutput::UpdateStatus`.
  6.  **Documentation:** Explain `AudioCaptureActor`, threading, message flow (UI `defer_msg` -> Coord -> Coord `send_msg` -> Capture -> `mpsc` -> ReceiverThread `defer_msg` -> Coord).
  7.  **Testing:** Manual: `cargo run`. Click Start/Stop. Verify status updates, logs show chunk arrivals at coordinator.

---

**Phase 4: Stub Transcriber Process & IPC Client**

- **Goal:** Create the `transcriber` stub process communication. Implement `TranscriptionClientActor` in `core` for process management and basic IPC.
- **Tasks:**
  1.  **IPC Types (`core/src/types.rs`):** Add `serde`, `serde_json`. Define `IpcAudioChunk`, `IpcTranscriptionResult`, `FinalTranscription(String)`.
  2.  **Transcriber Stub (`transcriber/src/main.rs`):** Add `serde`, `serde_json`. Loop reading stdin lines, deserialize `IpcAudioChunk`. Serialize dummy `IpcTranscriptionResult`, print line to stdout, flush.
  3.  **Transcription Client Actor (`core/src/transcriber_client.rs`):**
      - Implement `Actor`. `init` takes `mpsc::Sender<FinalTranscription>`.
      - `handle_msg`: Handles `IpcAudioChunk`.
      - In `init` or on first message: Spawn `transcriber` process (`std::process::Command`). Spawn thread for stdout reading (deserialize `IpcTranscriptionResult`, send via `mpsc::Sender<FinalTranscription>`). Spawn thread/use message queue for stdin writing (receive `IpcAudioChunk`, serialize, write line, flush).
  4.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - `AppCoordinatorActor`:
        - `init`: Create `mpsc::channel::<FinalTranscription>()`. Spawn `TranscriptionClientActor`, pass sender. Store handle. Spawn thread to receive `FinalTranscription` from channel and `self.defer_msg(InternalTranscriptionResult(result))`.
        - Define `CoordinatorMsg { ..., InternalAudioChunk(AudioChunk), InternalTranscriptionResult(FinalTranscription) }`.
        - Modify `InternalAudioChunk` handler: Create `IpcAudioChunk`. `self.transcriber_client_actor.send_msg(ipc_chunk)` (actor-to-actor).
        - Handle `InternalTranscriptionResult`: Log text. Send `AppOutput::UpdateTranscription` to UI.
  5.  **UI Integration (`src/ui.rs`):** Add `gtk::TextView`/`Label`. Define `AppOutput::UpdateTranscription(String)`. Coordinator sends it. `AppModel::update` handles it.
  6.  **Documentation:** Specify IPC protocol (JSON Lines). Explain `TranscriptionClientActor` threads. Detail stub behavior. Message flow.
  7.  **Testing:** Manual: `cargo run`. Start listening. Verify logs show full flow, UI shows dummy transcriptions.

---

**Phase 5: Vosk Integration in Transcriber**

- **Goal:** Replace the `transcriber` stub with actual Vosk model loading and recognition.
- **Tasks:**
  1.  **Dependencies (`transcriber/Cargo.toml`):** Add `vosk`.
  2.  **Configuration (`transcriber/src/main.rs`):** Use args/config file for model path, sample rate.
  3.  **Vosk Initialization:** Load `vosk::Model`, create `vosk::Recognizer`. Handle errors.
  4.  **Recognition Loop:** Modify main loop: Deserialize `IpcAudioChunk`. Call `recognizer.accept_waveform()`. Check `partial_result()`/`final_result()`. Serialize actual `IpcTranscriptionResult`, write to stdout, flush. Handle Vosk errors.
  5.  **Model Download Script:** Adapt/create script for Vosk models. Document.
  6.  **Documentation:** Specify `transcriber` config/args. Model download instructions.
  7.  **Testing:** Independent test: Pipe sample IPC data to `transcriber`, check stdout for correct transcription JSON.

---

**Phase 6: End-to-End Transcription Flow & Basic Config**

- **Goal:** Connect real audio to the real transcriber, display results in UI. Implement basic model path config loading in `core`.
- **Tasks:**
  1.  **Core Configuration (`core/src/config.rs`):** Add `config-rs`, `serde`. Define `Settings { model_path: Option<String> }`. Implement `load_config()`, `save_config()`.
  2.  **Pass Config (`core/src/coordinator.rs`, `core/src/transcriber_client.rs`):** Coordinator loads config. Pass necessary info to `TranscriptionClientActor`. Client passes `--model-path` arg when spawning `transcriber`.
  3.  **UI Settings Stub (`src/settings.rs`, `src/ui.rs`):** Create placeholder settings dialog component. Add "Settings" menu item to show it.
  4.  **Refine UI Output (`src/ui.rs`):** Ensure transcription `TextView` updates correctly.
  5.  **Documentation:** Document config file location, `model_path` setting.
  6.  **Testing:** Manual: Ensure model/config exist. `cargo run`. Start listening, speak. Verify _actual_ transcriptions in UI. Verify `transcriber` spawned with correct args. Open (empty) Settings dialog.

---

**Phase 7: Audio Pre-processing (Denoise, VAD)**

- **Goal:** Implement `AudioProcessingActor` in `core` with denoising and VAD.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add `nnnoiseless`, VAD crate.
  2.  **Audio Processing Actor (`core/src/audio_processing.rs`):** Define `AudioProcessingActor`. `init` takes input `mpsc::Receiver<AudioChunk>`, output `mpsc::Sender<AudioChunk>`. Run loop in `std::thread`. Receive chunk, denoise, apply VAD, forward chunk via output sender if speech detected (with gating).
  3.  **Pipeline Integration (`core/src/coordinator.rs`):** Update `AppCoordinatorActor::init`: Create two `mpsc` channels. Wire `AudioCaptureActor -> capture_to_process_tx/rx -> AudioProcessingActor -> process_to_client_tx/rx -> TranscriptionClientActor`.
  4.  **Configuration (`core/src/config.rs`, `src/settings.rs`):** Add settings (enable/disable denoise/VAD, VAD mode). Update load/save. Pass config to `AudioProcessingActor`. Add GTK controls to settings UI (`src`).
  5.  **Documentation:** Explain denoise/VAD. Update pipeline diagram. Document config options.
  6.  **Testing:** Manual: Test transcription quality with options enabled/disabled via settings. Verify VAD stops sending chunks during silence.

---

**Phase 8: Activation Logic (Hotkey & Wake Word)**

- **Goal:** Implement hotkey and wake word activation, including silence detection for stopping.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add hotkey library (`inputbot`), wake word library (`porcupine-rs`).
  2.  **State Management (`core/src/types.rs`, `core/src/coordinator.rs`):** Define `AppState` enum. Add `current_state` field to `AppCoordinatorActor`.
  3.  **Hotkey Listener (`core/src/activation/hotkey.rs`):** Create `run_hotkey_listener`. Run in `std::thread`. Use `inputbot` to bind key. On event, send `CoordinatorMsg::HotkeyTriggered` to coordinator (needs sender/handle passed from `core` - likely using `External<M>` obtained from `Stakker::external()` or an `mpsc` channel).
  4.  **Wake Word Detection (`core/src/activation/wakeword.rs`, `core/src/audio_processing.rs`):** Add wake word engine to `AudioProcessingActor`. Feed raw audio. On detection, send `CoordinatorMsg::WakeWordDetected` to coordinator.
  5.  **Silence Detection (`core/src/audio_processing.rs`, `core/src/coordinator.rs`):** Use VAD output in `AudioProcessingActor`. Track silence. If threshold exceeded, send `CoordinatorMsg::SilenceTimeout` to coordinator.
  6.  **Coordinator Logic (`core/src/coordinator.rs`):**
      - `init`: Spawn hotkey listener thread, pass sender/handle.
      - `handle_msg`: Handle `HotkeyTriggered`, `WakeWordDetected`, `SilenceTimeout`. Implement state transitions. Start/Stop `AudioCaptureActor` using `send_msg`. Signal UI state changes.
  7.  **Configuration (`core/src/config.rs`, `src/settings.rs`):** Add settings (activation mode, hotkey, WW model path, enable WW, silence timeout). Update load/save. Add GTK controls to settings UI (`src`).
  8.  **Wake Word Model Download:** Provide script/instructions.
  9.  **Documentation:** Explain activation logic, state machine. Document config. WW model instructions.
  10. **Testing:** Manual: Test hotkey start/stop. Test WW start + silence stop. Test changing modes. Verify UI status updates.

---

**Phase 9: Output Implementation (Typing)**

- **Goal:** Implement `OutputActor` in `core` using `enigo` to type transcriptions.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add `enigo`.
  2.  **Output Actor (`core/src/output.rs`):** Define `OutputActor`. `init`: Create `Enigo::new()`. `handle_msg` for `FinalTranscription`: Call `enigo.text()`. Handle errors.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):** `init`: Spawn `OutputActor`, store handle. Modify `InternalTranscriptionResult` handler: If state implies typing, `self.output_actor_handle.send_msg(FinalTranscription(result.0))` (actor-to-actor).
  4.  **Documentation:** Explain `OutputActor`.
  5.  **Testing:** Manual: Activate, speak. Verify text typed into focused window.

---

**Phase 10: Command Handling**

- **Goal:** Implement command definition, parsing, and execution.
- **Tasks:**
  1.  **Command Configuration (`core/src/config.rs`):** Define `CommandAction` enum (`Type`, `Exec`), command structs. Add `commands: HashMap<String, CommandAction>` to `Settings`. Update load/save. Use TOML.
  2.  **Command Logic (`core/src/command.rs`, `core/src/output.rs`):** Create `parse_and_execute` function. Logic: Check transcription against command triggers. If match: Substitute args into template. `Exec`: `std::process::Command::spawn()` in a thread. `Type`: Construct text, `output_actor_handle.send_msg(...)`. If no match: `output_actor_handle.send_msg(...)` with original text.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):** Modify `InternalTranscriptionResult` handler: Call `command::parse_and_execute(...)` instead of directly sending to `OutputActor`.
  4.  **UI Configuration (`src/settings.rs`):** Add GTK section to settings UI for command management (view/add/edit/remove). Load/save via `core::config`.
  5.  **Documentation:** Document command config syntax, actions, templating, UI.
  6.  **Testing:** Manual: Define commands. Test speaking them. Verify execution. Test UI for editing commands.

---

**Phase 11: Error Handling & Robustness**

- **Goal:** Systematically improve error handling, logging, and recovery.
- **Tasks:**
  1.  **Error Types (`core/src/errors.rs`):** Define custom errors using `thiserror`.
  2.  **Logging:** Configure file logging. Add more `span!`s, detailed event/error logging.
  3.  **Transcriber Resilience (`core/src/transcriber_client.rs`):** Monitor child process. On unexpected exit, log, signal coordinator, implement optional auto-restart. Handle pipe errors.
  4.  **Actor Error Handling (`core`):** Use `Result`. Handle errors from `send_msg`/`defer_msg`. Report critical errors to coordinator.
  5.  **User Feedback (`src/ui.rs`, `core/src/coordinator.rs`):** Coordinator formats errors. Send `AppOutput::ShowError(String)`. UI shows `gtk::MessageDialog`/`InfoBar`.
  6.  **Configuration Validation (`core/src/config.rs`):** Validate config post-load. Report errors.
  7.  **Documentation:** Describe logging setup, common errors, troubleshooting.
  8.  **Testing:** Manual: Kill transcriber. Provide invalid config. Verify errors reported gracefully in UI/logs. Check log files.

---

**Phase 12: Packaging & Documentation**

- **Goal:** Create distributable packages, write final documentation.
- **Tasks:**
  1.  **Build Scripts (`scripts/`):** Create scripts (`Makefile`, `justfile`, shell) or use `cargo-dist`. Compile main app, transcriber. Bundle executables, assets, default config. Handle GTK runtime, `tauri-plugin-system-tray` resources. Provide Vosk model instructions. Create platform installers/archives.
  2.  **Refine Icons & Assets:** Create proper icons.
  3.  **User Documentation:** Write comprehensive User Guide (Install, Setup, Usage, Config/Commands, Troubleshooting).
  4.  **Developer Documentation:** Finalize `README.md`. Add `CONTRIBUTING.md`. `cargo doc`. Architecture diagrams.
  5.  **Licensing:** Add `LICENSE` file.
  6.  **Final Testing:** Test installation/functionality _from packages_ on clean VMs/machines (Win, macOS, Linux). Regression test all features.
  7.  **Release:** Tag version. Upload packages.
