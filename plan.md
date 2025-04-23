**Assumptions:**

1.  **UI Framework:** **GTK4-rs / Relm4** for the main application window and UI elements (Root Crate: `src/`).
2.  **Core Logic:** **Ractor** actors, audio pipeline, state management, command handling, IPC client, activation logic, configuration handling resides in the **`core/`** library crate.
3.  **System Tray:** **`tauri-plugin-system-tray`** integrated in the Root Crate (`src/`), interacting with the `core` crate.
4.  **Actor Framework:** **Ractor** used within the `core/` crate.
5.  **Concurrency:** Standard **multi-threading** (`std::thread`, `std::sync::mpsc`) used within the `core/` crate alongside Ractor actors.
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
      - `core/`: Library - Ractor actors, logic, audio, state, config.
      - `transcriber/`: Binary - Vosk process.

---

**Phase 1: Project Setup & Basic UI Shell**

- **Goal:** Initialize crates, dependencies, basic GTK window, and system tray icon. No actors or audio yet.
- **Tasks:**
  1.  **Workspace Setup:** Create `whisperkey/Cargo.toml` defining the workspace.
  2.  **Root Crate (`src/`) Setup:** Add deps (`gtk4`, `relm4`, `tracing`, `tracing-subscriber`, `tauri-plugin-system-tray`, `whisperkey-core`). Basic `main.rs` with GTK/Relm4 structure, empty `AppModel`, `AppWidgets`, empty `AppInput`/`AppOutput`. Run Relm4 app.
  3.  **Core Crate (`core/`) Setup:** `[lib]`. Add deps (`ractor = "0.13"`, `tracing`). Placeholder function `core_hello()`.
  4.  **Transcriber Crate (`transcriber/`) Setup:** Add deps (`tracing`, `tracing-subscriber`). `main.rs` logs start message.
  5.  **Basic System Tray (`src/tray.rs`):** Use `tauri-plugin-system-tray` for icon and basic Quit menu item. Handle threading.
  6.  **Documentation:** `README.md`: Structure, deps, build command.
  7.  **Testing:** Manual: `cargo run`. Verify empty GTK window, tray icon, Quit works. Logs show init messages. Run transcriber stub.

---

**Phase 2: Ractor & Core Initialization**

- **Goal:** Set up Ractor actors in `core`, initialize them from `src`, establish UI -> Core message passing.
- **Tasks:**
  1.  **Ractor Setup (`core/src/lib.rs`):** Define `Coordinator` actor struct and message types. Implement the `Actor` trait for `Coordinator`. Define `init_core_actors` function returning `CoreHandles { coordinator: ActorRef<CoordinatorMsg> }`.
  2.  **Ractor Integration (`src/main.rs`):** Create a handle to the coordinator actor by calling `core::init_core_actors`. Store `CoreHandles`.
  3.  **Basic UI->Core Communication (`src/ui.rs`, `core/src/lib.rs`):**
      - Add "Test Core" GTK button (`src/`).
      - Define `AppInput::TestCore` (`src/`).
      - Define `CoordinatorMsg::HandleTest` (`core/`).
      - Button click handler (`src/`) sends `AppInput::TestCore`.
      - `AppModel::update` (`src/`) handles `AppInput::TestCore`: use `core_handles.coordinator.send_message(CoordinatorMsg::HandleTest).unwrap();` (using `send_message` to communicate with the actor).
      - `Coordinator::handle` (`core/`) processes `CoordinatorMsg::HandleTest`: logs message.
  4.  **Documentation:** Explain Ractor initialization, UI-to-Core message flow.
  5.  **Testing:** Manual: `cargo run`. Click "Test Core". Verify coordinator log message appears.

---

**Phase 3: Audio Capture Implementation**

- **Goal:** Implement `AudioCaptureActor` in `core` using `cpal` and `std::thread`, send raw audio chunks to the coordinator.
- **Tasks:**
  1.  **Audio Types (`core/src/types.rs`):** `AudioChunk(Vec<f32>)`, `AudioCaptureMsg { Start, Stop, Shutdown }`.
  2.  **Audio Capture Actor (`core/src/audio_capture.rs`):** Implement `Actor` trait for `AudioCapture`. State will hold `Option<JoinHandle>` for the capture thread. `pre_start` initializes the actor. `handle` processes `AudioCaptureMsg::Start`/`Stop`/`Shutdown`. `Start` spawns `std::thread` for `cpal` stream callback. Audio callback sends chunks back to coordinator using `ActorRef::send_message`.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):**
  - Define `CoordinatorMsg { StartListening, StopListening, AudioChunk(AudioChunk) }`.
  - Implement `Actor` trait for `Coordinator`:
    - `pre_start`: Spawn `AudioCaptureActor` using `Actor::spawn`, store `ActorRef<AudioCaptureMsg>`.
    - `handle`: Process `StartListening`/`StopListening`: `self.audio_capture_actor.send_message(AudioCaptureMsg::Start/Stop).unwrap()`.
    - Handle `AudioChunk`: Log arrival.
  4.  **UI Integration (`src/ui.rs`):**
  - Add "Start/Stop Listening" buttons, Status label.
  - Define `AppInput { StartListening, StopListening }`, `AppOutput { UpdateStatus(String) }`.
  - Button clicks send `AppInput`.
  - `AppModel::update` handles inputs: `core_handles.coordinator.send_message(CoordinatorMsg::StartListening).unwrap()`.
  - Coordinator (`core`) sends status updates back to UI (via a channel or Relm4 sender passed during init).
  5.  **Documentation:** Explain `AudioCaptureActor`, threading, message flow (UI -> Coordinator -> AudioCaptureActor -> Thread -> Coordinator).
  6.  **Testing:** Manual: `cargo run`. Click Start/Stop. Verify status updates, logs show chunk arrivals at coordinator.

---

**Phase 4: Stub Transcriber Process & IPC Client**

- **Goal:** Create the `transcriber` stub process communication. Implement `TranscriberActor` in `core` for process management and basic IPC.
- **Tasks:**
  1.  **IPC Types (`core/src/types.rs`):** Add `serde`, `serde_json`. Define `IpcAudioChunk`, `IpcTranscriptionResult`, `FinalTranscription(String)`.
  2.  **Transcriber Stub (`transcriber/src/main.rs`):** Add `serde`, `serde_json`. Loop reading stdin lines, deserialize `IpcAudioChunk`. Serialize dummy `IpcTranscriptionResult`, print line to stdout, flush.
  3.  **Transcriber Actor (`core/src/transcriber.rs`):**
      - Define messages: `TranscriberMsg { ProcessAudioChunk(AudioChunk), Shutdown }`.
      - Implement `Actor` trait for `Transcriber`. State will hold process handle and communication channels.
      - `pre_start`: Spawn `transcriber` process (`std::process::Command`). Spawn threads for stdin/stdout communications, with access to `ActorRef<CoordinatorMsg>` to send results back to coordinator.
      - `handle`: Process `ProcessAudioChunk` by sending data to transcriber process via stdin. Handle `Shutdown` by terminating the process.
  4.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - Extend `CoordinatorMsg` with `TranscriptionResult(FinalTranscription)`.
      - Modify `Coordinator`:
        - `pre_start`: Spawn `TranscriberActor` using `Actor::spawn`, store `ActorRef<TranscriberMsg>`.
        - Modify `AudioChunk` handler: Forward chunk to transcriber: `self.transcriber.send_message(TranscriberMsg::ProcessAudioChunk(chunk)).unwrap()`.
        - Handle `TranscriptionResult`: Log text. Send `AppOutput::UpdateTranscription` to UI.
  5.  **UI Integration (`src/ui.rs`):** Add `gtk::TextView`/`Label`. Define `AppOutput::UpdateTranscription(String)`. `AppModel::update` handles it.
  6.  **Documentation:** Specify IPC protocol (JSON Lines). Explain `TranscriberActor` threads. Detail stub behavior. Message flow.
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
  2.  **Pass Config (`core/src/coordinator.rs`, `core/src/transcriber.rs`):** Coordinator loads config. Pass necessary info to `TranscriberActor`. Client passes `--model-path` arg when spawning `transcriber`.
  3.  **UI Settings Stub (`src/settings.rs`, `src/ui.rs`):** Create placeholder settings dialog component. Add "Settings" menu item to show it.
  4.  **Refine UI Output (`src/ui.rs`):** Ensure transcription `TextView` updates correctly.
  5.  **Documentation:** Document config file location, `model_path` setting.
  6.  **Testing:** Manual: Ensure model/config exist. `cargo run`. Start listening, speak. Verify _actual_ transcriptions in UI. Verify `transcriber` spawned with correct args. Open (empty) Settings dialog.

---

**Phase 7: Audio Pre-processing (Denoise, VAD)**

- **Goal:** Implement `AudioProcessorActor` in `core` with denoising and VAD.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add `nnnoiseless`, VAD crate.
  2.  **Audio Processor Actor (`core/src/audio_processor.rs`):**
      - Define messages: `AudioProcessorMsg { ProcessChunk(AudioChunk), Shutdown }`.
      - Implement `Actor` trait for `AudioProcessor`. State might include denoising/VAD state.
      - `pre_start`: Initialize audio processing components.
      - `handle`: Process `ProcessChunk` by applying denoising/VAD, then forward to transcriber via `self.transcriber.send_message(TranscriberMsg::ProcessAudioChunk(processed_chunk)).unwrap()`.
  3.  **Pipeline Integration (`core/src/coordinator.rs`):** Update `Coordinator::pre_start`: Spawn `AudioProcessorActor` and `TranscriberActor`, store their references. Modify `AudioChunk` handler: Forward to audio processor: `self.audio_processor.send_message(AudioProcessorMsg::ProcessChunk(chunk)).unwrap()`.
  4.  **Configuration (`core/src/config.rs`, `src/settings.rs`):** Add settings (enable/disable denoise/VAD, VAD mode). Update load/save. Pass config to `AudioProcessorActor`. Add GTK controls to settings UI (`src`).
  5.  **Documentation:** Explain denoise/VAD. Update pipeline diagram to reflect actor message flow. Document config options.
  6.  **Testing:** Manual: Test transcription quality with options enabled/disabled via settings. Verify VAD stops sending chunks during silence.

---

**Phase 8: Activation Logic (Hotkey & Wake Word)**

- **Goal:** Implement hotkey and wake word activation, including silence detection for stopping.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add hotkey library (`inputbot`), wake word library (`porcupine-rs`).
  2.  **State Management (`core/src/types.rs`, `core/src/coordinator.rs`):** Define `AppState` enum. Add `current_state` field to `Coordinator` state.
  3.  **Hotkey Listener (`core/src/activation/hotkey.rs`):** Create `run_hotkey_listener`. Run in `std::thread`. Use `inputbot` to bind key. On event, use `ActorRef::send_message` to send `CoordinatorMsg::HotkeyTriggered` to coordinator.
  4.  **Wake Word Detection (`core/src/activation/wakeword.rs`, `core/src/audio_processor.rs`):** Add wake word detection to `AudioProcessorActor`. Feed raw audio. On detection, send `CoordinatorMsg::WakeWordDetected` to coordinator actor.
  5.  **Silence Detection (`core/src/audio_processor.rs`, `core/src/coordinator.rs`):** Use VAD output in `AudioProcessorActor`. Track silence. If threshold exceeded, send `CoordinatorMsg::SilenceTimeout` to coordinator.
  6.  **Coordinator Logic (`core/src/coordinator.rs`):**
      - Add to `pre_start`: Create and store hotkey thread, pass ActorRef to hotkey listener.
      - Extend `handle`: Process `HotkeyTriggered`, `WakeWordDetected`, `SilenceTimeout`. Implement state transitions. Start/Stop actors using `send_message`. Signal UI state changes.
  7.  **Configuration (`core/src/config.rs`, `src/settings.rs`):** Add settings (activation mode, hotkey, WW model path, enable WW, silence timeout). Update load/save. Add GTK controls to settings UI (`src`).
  8.  **Wake Word Model Download:** Provide script/instructions.
  9.  **Documentation:** Explain activation logic, state machine. Document config. WW model instructions.
  10. **Testing:** Manual: Test hotkey start/stop. Test WW start + silence stop. Test changing modes. Verify UI status updates.

---

**Phase 9: Output Implementation (Typing)**

- **Goal:** Implement `OutputActor` in `core` using `enigo` to type transcriptions.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):** Add `enigo`.
  2.  **Output Actor (`core/src/output.rs`):**
      - Define messages: `OutputMsg { TypeText(String), Shutdown }`.
      - Implement `Actor` trait for `Output`. State will contain `Enigo` instance.
      - `pre_start`: Create `Enigo::new()`.
      - `handle`: Process `TypeText` by calling `enigo.text()`. Handle errors.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):** `pre_start`: Spawn `OutputActor`, store reference. Modify `TranscriptionResult` handler: If state implies typing, `self.output.send_message(OutputMsg::TypeText(result.0)).unwrap()`.
  4.  **Documentation:** Explain `OutputActor`.
  5.  **Testing:** Manual: Activate, speak. Verify text typed into focused window.

---

**Phase 10: Command Handling**

- **Goal:** Implement command definition, parsing, and execution.
- **Tasks:**
  1.  **Command Configuration (`core/src/config.rs`):** Define `CommandAction` enum (`Type`, `Exec`), command structs. Add `commands: HashMap<String, CommandAction>` to `Settings`. Update load/save. Use TOML.
  2.  **Command Logic (`core/src/command.rs`, `core/src/coordinator.rs`):** Create `parse_command` function in coordinator. Logic: Check transcription against command triggers. If match: Substitute args into template. `Exec`: Use coordinator to spawn a thread for `std::process::Command::spawn()`. `Type`: Construct text, `self.output.send_message(OutputMsg::TypeText(...)).unwrap()`. If no match: `self.output.send_message(OutputMsg::TypeText(...)).unwrap()` with original text.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):** Modify `TranscriptionResult` handler: Call `self.parse_command(...)` with the text directly.
  4.  **UI Configuration (`src/settings.rs`):** Add GTK section to settings UI for command management (view/add/edit/remove). Load/save via `core::config`.
  5.  **Documentation:** Document command config syntax, actions, templating, UI.
  6.  **Testing:** Manual: Define commands. Test speaking them. Verify execution. Test UI for editing commands.

---

**Phase 11: Error Handling & Robustness**

- **Goal:** Systematically improve error handling, logging, and recovery.
- **Tasks:**
  1.  **Error Types (`core/src/errors.rs`):** Define custom errors using `thiserror`.
  2.  **Logging:** Configure file logging. Add more detailed event/error logging.
  3.  **Transcriber Resilience (`core/src/transcriber.rs`):** Monitor child process. On unexpected exit, notify coordinator, implement optional auto-restart. Handle pipe errors.
  4.  **Actor Error Handling (`core`):** Use `Result` for message handling. Handle errors from `send_message` (unwrapping or checking results). Report critical errors to coordinator.
  5.  **User Feedback (`src/ui.rs`, `core/src/coordinator.rs`):** Coordinator formats errors. Send `AppOutput::ShowError(String)` to UI thread. UI shows `gtk::MessageDialog`/`InfoBar`.
  6.  **Configuration Validation (`core/src/config.rs`):** Validate config post-load. Report errors.
  7.  **Supervision and Actor Lifecycle Management:** Leverage Ractor's supervision model for actor crash recovery.
  8.  **Documentation:** Describe logging setup, common errors, troubleshooting. Document supervision strategy.
  9.  **Testing:** Manual: Kill transcriber. Provide invalid config. Verify errors reported gracefully in UI/logs. Check log files.

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
