## Detailed Development Plan for WhisperKey (12 Phases)

**Assumptions (Reiterated):**

1.  **UI Framework:** **GTK4-rs / Relm4** for the main application window and UI elements (Root Crate: `src/`).
2.  **Core Logic:** **Stakker** actors, audio pipeline, state management, command handling, IPC client, activation logic, configuration handling resides in the **`core/`** library crate.
3.  **System Tray:** **`tauri-plugin-system-tray`** integrated in the Root Crate (`src/`), interacting with the `core` crate.
4.  **Actor Framework:** **Stakker** used within the `core/` crate.
5.  **Concurrency:** Standard **multi-threading** (`std::thread`, `std::sync::mpsc`) used within the `core/` crate.
6.  **Transcription Backend:** **Vosk-rs** running in a separate process (`transcriber/` crate).
7.  **Transcription Process IPC:** stdin/stdout using **JSON Lines** initially for easier debugging, potentially switching to `bincode` later for performance.
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
  1.  **Workspace Setup:**
      - Create `whisperkey/Cargo.toml` defining the workspace `members = [".", "core", "transcriber"]`.
  2.  **Root Crate (`whisperkey/src/`) Setup:**
      - `Cargo.toml`: Add deps: `gtk4`, `relm4`, `tracing`, `tracing-subscriber`, `tauri-plugin-system-tray`, `whisperkey-core = { path = "../core" }`.
      - `main.rs`:
        - Initialize `tracing` subscriber (e.g., to console).
        - Initialize GTK.
        - Set up basic Relm4 application structure (`relm4::main_application`).
        - Define `AppModel` struct (empty for now).
        - Define `AppWidgets` struct holding basic `gtk::Window`, `gtk::Box`.
        - Define empty `AppInput`, `AppOutput` enums.
        - Implement basic `relm4::Component` trait for `AppModel`.
        - Create the main window (visible, with title).
        - Run the Relm4 application.
  3.  **Core Crate (`whisperkey/core/`) Setup:**
      - `Cargo.toml`: `[lib]`, Add deps: `stakker`, `tracing`.
      - `lib.rs`: Define a basic placeholder function, e.g., `pub fn core_hello() { tracing::info!("Core library initialized"); }`. Call this from `src/main.rs` after initializing Stakker (in next phase) to verify linking.
  4.  **Transcriber Crate (`whisperkey/transcriber/`) Setup:**
      - `Cargo.toml`: Add deps: `tracing`, `tracing-subscriber`.
      - `main.rs`: Initialize `tracing`, print a "Transcriber stub started" message, and exit or sleep indefinitely for now.
  5.  **Basic System Tray (`src/tray.rs`, called from `src/main.rs`):**
      - Define `fn setup_system_tray()`.
      - Use `tauri-plugin-system-tray` to create a basic icon (needs an asset).
      - Create a simple menu: "Show/Hide" (no action yet), "Separator", "Quit" (connect to `gtk::main_quit` or Relm4 shutdown).
      - Handle potential threading requirements for the tray's event loop if needed.
  6.  **Documentation:**
      - `README.md`: Initial project structure, dependencies (mention GTK dev libraries needed), basic build command (`cargo build`).
  7.  **Testing:**
      - Manual: `cargo run`. Verify:
        - Empty GTK window appears with the correct title.
        - System tray icon appears.
        - Tray menu shows "Show/Hide", "Quit".
        - Clicking "Quit" closes the application.
        - Console logs show initialization messages from `src` and potentially `core`.
        - Run `cargo run -p whisperkey-transcriber` - verify console message.

---

**Phase 2: Stakker & Core Initialization**

- **Goal:** Introduce Stakker, define minimal actors in `core`, initialize them from `src`, establish UI -> Core message passing.
- **Tasks:**
  1.  **Stakker Setup (`core/src/lib.rs`):**
      - Define basic actor placeholders:
        - `pub struct AppCoordinator { /* ... */ }`
        - Implement `impl Actor for AppCoordinator { type Ret = (); fn init(...) {} fn handle_msg(...) {} ... }`
      - Define core initialization function:
        - `pub struct CoreHandles { pub coordinator: Actor<AppCoordinator> }`
        - `pub fn init_core_actors(stakker: &mut Stakker) -> CoreHandles { let coord = stakker.spawn(|| ...); CoreHandles { coordinator: coord } }`
  2.  **Stakker Integration (`src/main.rs`):**
      - Create `Stakker` instance in `main`.
      - Call `core::init_core_actors(&mut stakker)` and store `CoreHandles` (e.g., in `AppModel` or pass sender to it).
      - Keep the `Stakker` instance running (e.g., needs `stakker.run(Duration::from_secs(u64::MAX));` if not driven by GTK loop, check Stakker docs for GTK integration best practices - might involve `stakker::idle()` calls).
  3.  **Basic UI->Core Communication (`src/ui.rs`, `core/src/lib.rs`):**
      - Add a "Test Core" button to the GTK window (`src/`).
      - Define `enum AppInput { TestCore }`.
      - Define `enum CoordinatorMsg { HandleTest }` in `core`.
      - In the button's `connect_clicked` handler (`src/`), send `AppInput::TestCore` to the Relm4 component.
      - In `AppModel::update` (`src/`), handle `AppInput::TestCore`: use the `CoreHandles` to `cast!(coordinator_actor, CoordinatorMsg::HandleTest)`.
      - In `AppCoordinator::handle_msg` (`core/`), handle `CoordinatorMsg::HandleTest`: `tracing::info!("Coordinator received TestCore message");`.
  4.  **Documentation:**
      - Explain Stakker initialization flow.
      - Document basic UI-to-Core message example.
  5.  **Testing:**
      - Manual: `cargo run`. Click "Test Core" button. Verify console log shows "Coordinator received TestCore message".

---

**Phase 3: Audio Capture Implementation**

- **Goal:** Implement `AudioCaptureActor` in `core` using `cpal` and `std::thread`, send raw audio chunks via `mpsc`.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):**
      - Add `cpal`.
  2.  **Audio Types (`core/src/types.rs`):**
      - `pub struct AudioChunk(pub Vec<f32>);`
      - `pub enum AudioCaptureCmd { Start, Stop }`
  3.  **Audio Capture Actor (`core/src/audio_capture.rs`):**
      - Define `struct AudioCaptureActor { stream: Option<cpal::Stream>, audio_tx: std::sync::mpsc::Sender<AudioChunk> }`
      - Implement `Actor` trait.
      - `init`: Takes `audio_tx` channel sender.
      - `handle_msg`: Handles `AudioCaptureCmd::Start` and `Stop`.
        - `Start`: Sets up `cpal` host, device, config. Spawns a `std::thread` for the input stream callback. Stores the `cpal::Stream` to allow stopping. Logs device info.
        - `Stop`: Drops the `cpal::Stream`, joins the thread if necessary.
      - The spawned thread's callback: Converts audio data (e.g., i16/u16 to f32), creates `AudioChunk`, sends via `audio_tx`. Handles potential send errors (log them).
  4.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - `AppCoordinatorActor`:
        - Create `std::sync::mpsc::channel::<AudioChunk>()` in `init`.
        - Spawn `AudioCaptureActor`, passing the sender. Store the `Actor<AudioCaptureActor>` handle.
        - Define `enum CoordinatorMsg { StartListening, StopListening, InternalAudioChunk(AudioChunk) }`.
        - Spawn a separate thread to receive from the `mpsc::Receiver<AudioChunk>` and `cast!` `InternalAudioChunk` messages back to self (to process on actor's thread).
        - Handle `StartListening`/`StopListening` messages: `cast!` `AudioCaptureCmd` to `AudioCaptureActor`.
        - Handle `InternalAudioChunk`: Log chunk arrival for now (e.g., `tracing::debug!("Received audio chunk, size: {}", chunk.0.len());`).
  5.  **UI Integration (`src/ui.rs`):**
      - Replace "Test Core" button with "Start Listening" and "Stop Listening".
      - Define `AppInput { StartListening, StopListening }`.
      - Connect buttons to send corresponding `AppInput` messages.
      - In `AppModel::update`, handle inputs by casting `CoordinatorMsg::StartListening`/`StopListening` to the coordinator actor.
      - Add a status label widget. Define `AppOutput { UpdateStatus(String) }`.
      - Modify `AppCoordinatorActor` (`core`) to send status updates (e.g., "Listening...", "Idle") back to the UI (requires a channel/sender passed from `src` to `core` during init, or use Relm4's sender directly if passed). Update status label in `AppModel::update` when `AppOutput::UpdateStatus` is received.
  6.  **Documentation:**
      - Explain `AudioCaptureActor` implementation, threading model, `cpal` usage.
      - Document communication flow: UI -> Coordinator -> Capture Actor -> mpsc -> Coordinator -> UI Status.
  7.  **Testing:**
      - Manual: `cargo run`.
        - Verify initial status is "Idle".
        - Click "Start Listening". Verify status changes to "Listening..." and console logs show audio chunks arriving at the coordinator. Check audio input device is logged.
        - Click "Stop Listening". Verify status changes to "Idle" and console logs stop showing chunk arrivals.

---

**Phase 4: Stub Transcriber Process & IPC Client**

- **Goal:** Create the `transcriber` stub process communication. Implement `TranscriptionClientActor` in `core` for process management and basic IPC.
- **Tasks:**
  1.  **IPC Types (`core/src/types.rs`):**
      - Add `serde` dependency to `core` and `transcriber`. Use `serde_json` for now.
      - `#[derive(Serialize, Deserialize, Debug)] pub struct IpcAudioChunk { timestamp: u64, data: Vec<f32> };`
      - `#[derive(Serialize, Deserialize, Debug)] pub struct IpcTranscriptionResult { text: String };`
      - `pub struct FinalTranscription(pub String);`
  2.  **Transcriber Stub (`transcriber/src/main.rs`):**
      - Add `serde`, `serde_json`.
      - Modify main loop:
        - Use `std::io::BufReader` for stdin.
        - Loop reading lines from stdin (`reader.read_line`).
        - Deserialize each line as `IpcAudioChunk` using `serde_json::from_str`. Log errors/success.
        - For every chunk received, construct a dummy `IpcTranscriptionResult { text: format!("Received chunk at {}", timestamp) }`.
        - Serialize result to JSON string using `serde_json::to_string`.
        - Print the JSON string to stdout, followed by a newline. Flush stdout.
        - Handle EOF/errors robustly.
  3.  **Transcription Client Actor (`core/src/transcriber_client.rs`):**
      - Define `struct TranscriptionClientActor { /* ... */ }`.
      - Requires path to `transcriber` executable (pass during init, maybe find relative to main exe).
      - `init`: Takes channel sender for `FinalTranscription` results.
      - `handle_msg`: Handles `IpcAudioChunk` messages.
      - In `init` or on first message:
        - Spawn the `transcriber` process using `std::process::Command`. Capture its stdin/stdout.
        - Store `Child` handle, `ChildStdin`, `ChildStdout`.
        - Spawn a thread to read stdout from the process:
          - Use `BufReader` on `child.stdout`.
          - Read lines, deserialize as `IpcTranscriptionResult`.
          - Send `FinalTranscription(result.text)` via the results channel.
          - Handle errors/EOF (log, potentially signal coordinator).
        - Spawn a thread (or use message passing within actor) to handle writing to stdin:
          - Receive `IpcAudioChunk` messages.
          - Serialize to JSON string. Write line to `child.stdin`. Flush.
          - Handle pipe errors.
  4.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - `AppCoordinatorActor`:
        - Create `mpsc::channel::<FinalTranscription>()` for results.
        - Spawn `TranscriptionClientActor`, passing the results sender. Store its handle.
        - Modify `handle_msg` for `InternalAudioChunk`:
          - Create `IpcAudioChunk` (add timestamp).
          - `cast!` the `IpcAudioChunk` message to `TranscriptionClientActor`.
        - Add `InternalTranscriptionResult(FinalTranscription)` message variant.
        - Spawn thread to receive `FinalTranscription` from results channel and `cast!` `InternalTranscriptionResult` back to self.
        - Handle `InternalTranscriptionResult`: Log the final text (`tracing::info!("Transcription: {}", result.0);`). Send text to UI via `AppOutput`.
  5.  **UI Integration (`src/ui.rs`):**
      - Add a `gtk::TextView` or `gtk::Label` for displaying the transcription.
      - Define `AppOutput::UpdateTranscription(String)`.
      - Update `AppCoordinatorActor` (`core`) to send `AppOutput::UpdateTranscription` when handling `InternalTranscriptionResult`.
      - In `AppModel::update` (`src`), handle `UpdateTranscription` and update the TextView/Label.
  6.  **Documentation:**
      - Specify IPC protocol (JSON Lines over stdin/stdout).
      - Explain `TranscriptionClientActor` design (process spawning, IO threads).
      - Detail `transcriber` stub behavior.
  7.  **Testing:**
      - Manual: `cargo run`.
        - Click "Start Listening".
        - Verify logs show audio chunks being sent to client actor, then IPC chunks being logged by client actor, then received by `transcriber` stub.
        - Verify UI transcription area shows dummy messages like "Received chunk at ..." appearing.
        - Click "Stop Listening".

---

**Phase 5: Vosk Integration in Transcriber**

- **Goal:** Replace the `transcriber` stub with actual Vosk model loading and recognition.
- **Tasks:**
  1.  **Dependencies (`transcriber/Cargo.toml`):**
      - Add `vosk`. Remove dummy logic dependencies if any.
  2.  **Configuration Handling (`transcriber/src/main.rs`):**
      - Use command-line arguments (`std::env::args`) or a simple config file mechanism (e.g., read `transcriber_config.toml`) to get Vosk model path and sample rate.
  3.  **Vosk Initialization:**
      - Load `vosk::Model`. Handle errors (log clearly).
      - Create `vosk::Recognizer` using the model and configured sample rate. Handle errors.
  4.  **Recognition Loop:**
      - Modify main loop:
        - Deserialize `IpcAudioChunk` from stdin.
        - Call `recognizer.accept_waveform(&chunk.data)`.
        - Check results:
          - Call `recognizer.partial_result()` -> If new partial text, serialize `IpcTranscriptionResult { text: partial_text }` and write to stdout (optional, for live feedback).
          - Call `recognizer.final_result()` -> If final result available, serialize `IpcTranscriptionResult { text: final_text }`, write to stdout.
        - Handle Vosk errors during recognition.
  5.  **Model Download Script:**
      - Adapt/use the existing `download-whisper-model.sh` script to download Vosk models instead, or write a new one. Document its use.
  6.  **Documentation:**
      - Specify `transcriber` command-line arguments or config file format.
      - Instructions for downloading/placing Vosk models.
  7.  **Testing:**
      - Independent Test:
        - Create a sample script/program that generates JSON Lines `IpcAudioChunk` data from a known WAV file.
        - Pipe this data to `cargo run -p whisperkey-transcriber -- --model-path /path/to/vosk-model`.
        - Verify the stdout contains JSON Lines `IpcTranscriptionResult` with the expected transcription text.

---

**Phase 6: End-to-End Transcription Flow & Basic Config**

- **Goal:** Connect real audio to the real transcriber, display results in UI. Implement basic model path config loading in `core`.
- **Tasks:**
  1.  **Core Configuration (`core/src/config.rs`):**
      - Add `config-rs`, `serde` deps to `core`.
      - Define `struct Settings { pub model_path: Option<String>, /* ... */ }`. Implement `Default`.
      - Implement `fn load_config() -> Settings` using `config-rs` (e.g., loading from `~/.config/whisperkey/config.toml`). Handle file not found (use defaults).
      - Implement `fn save_config(settings: &Settings)`.
  2.  **Pass Config to Client Actor (`core/src/coordinator.rs`, `core/src/transcriber_client.rs`):**
      - Load config in `AppCoordinatorActor::init`.
      - Pass necessary parts (e.g., resolved transcriber path, potentially model path if client needs to pass it as arg) to `TranscriptionClientActor` during init.
      - Modify `TranscriptionClientActor` to potentially pass `--model-path` argument when spawning `transcriber` process, based on config.
  3.  **UI Settings Stub (`src/settings.rs`, `src/ui.rs`):**
      - Create a placeholder settings dialog (new Relm4 component).
      - Add a "Settings" menu item to the tray icon/main window menu.
      - On click, show the (empty) settings dialog.
      - Define `AppInput::ShowSettings`.
  4.  **Refine UI Output (`src/ui.rs`):**
      - Ensure the transcription `TextView` handles accumulating text correctly (if desired) or just displays the latest final result.
  5.  **Documentation:**
      - Document default config file location and basic `model_path` setting.
  6.  **Testing:**
      - Manual:
        - Ensure a Vosk model exists and `config.toml` points to it (create manually if needed).
        - `cargo run`.
        - Click "Start Listening". Speak clearly.
        - Verify _actual_ transcriptions appear in the UI TextView.
        - Verify `transcriber` process is spawned with correct `--model-path` argument (check logs or process monitor).
        - Click "Stop Listening".
        - Open Settings dialog (it will be empty).

---

**Phase 7: Audio Pre-processing (Denoise, VAD)**

- **Goal:** Implement `AudioProcessingActor` in `core` with denoising and VAD.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):**
      - Add `nnnoiseless`.
      - Add a VAD crate (e.g., `webrtc-vad-sync` or `vad`). Research best option for sync Rust.
  2.  **Audio Processing Actor (`core/src/audio_processing.rs`):**
      - Define `struct AudioProcessingActor { /* denoiser_state, vad_state */ }`.
      - Requires audio format info (sample rate) and VAD settings (aggressiveness).
      - `init`: Takes input `mpsc::Receiver<AudioChunk>` and output `mpsc::Sender<AudioChunk>`.
      - Run processing loop in a dedicated `std::thread::spawn`.
      - Loop:
        - Receive `AudioChunk` from input channel.
        - Apply denoising using `nnnoiseless` (adapt `denoise_audio` function).
        - Apply VAD: Check if chunk likely contains speech.
        - State machine: Only forward chunks when VAD indicates speech (implement simple hangover/noise gating).
        - If forwarding, send denoised `AudioChunk` via output channel.
  3.  **Pipeline Integration (`core/src/coordinator.rs`):**
      - Update `AppCoordinatorActor::init`:
        - Create two `mpsc` channels: `capture_to_process_tx/rx`, `process_to_client_tx/rx`.
        - Pass `capture_to_process_tx` to `AudioCaptureActor`.
        - Spawn `AudioProcessingActor`, passing `capture_to_process_rx` and `process_to_client_tx`.
        - Pass `process_to_client_rx` to `TranscriptionClientActor` (instead of the direct capture channel).
  4.  **Configuration (`core/src/config.rs`, `src/settings.rs`):**
      - Add settings for enabling/disabling denoise/VAD, VAD aggressiveness to `struct Settings`.
      - Update `load_config`, `save_config`.
      - Pass relevant config to `AudioProcessingActor` init.
      - Add GTK CheckButtons/ComboBoxes to settings UI (`src`) to control these options. Connect them to load/save via `core::config`.
  5.  **Documentation:**
      - Explain denoising and VAD implementation.
      - Update audio pipeline diagram.
      - Document new config options.
  6.  **Testing:**
      - Manual: `cargo run`.
        - Test transcription quality with denoise/VAD enabled/disabled via settings UI.
        - Verify VAD stops sending chunks during silence (check logs or lack of transcription updates).

---

**Phase 8: Activation Logic (Hotkey & Wake Word)**

- **Goal:** Implement hotkey and wake word activation, including silence detection for stopping.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):**
      - Add hotkey library (e.g., `inputbot`).
      - Add wake word library (e.g., `porcupine-rs`, requires model files).
  2.  **State Management (`core/src/types.rs`, `core/src/coordinator.rs`):**
      - Define `enum AppState { Idle, ListeningWW, RecordingByKey, RecordingByWW, Stopping }`.
      - `AppCoordinatorActor`: Add `current_state: AppState` field. Implement state transition logic.
  3.  **Hotkey Listener (`core/src/activation/hotkey.rs`):**
      - Create `fn run_hotkey_listener(coordinator_sender: ??? /* Stakker External<M> or mpsc Sender */)`
      - Run in `std::thread::spawn`.
      - Use `inputbot` or similar to bind configured key(s).
      - On key press/release (or toggle): Send message (e.g., `CoordinatorMsg::HotkeyTriggered`) to coordinator.
  4.  **Wake Word Detection (`core/src/activation/wakeword.rs`, `core/src/audio_processing.rs`):**
      - Add wake word engine instance (e.g., `porcupine::Porcupine`) to `AudioProcessingActor`. Requires model path from config.
      - Feed _raw_ (pre-denoise/VAD) audio chunks to the engine.
      - If wake word detected: Send message (e.g., `CoordinatorMsg::WakeWordDetected`) to coordinator.
  5.  **Silence Detection (`core/src/audio_processing.rs`, `core/src/coordinator.rs`):**
      - Use VAD output in `AudioProcessingActor`. Track consecutive silent chunks when in a recording state.
      - If silence exceeds threshold (configurable): Send message (e.g., `CoordinatorMsg::SilenceTimeout`) to coordinator.
  6.  **Coordinator Logic (`core/src/coordinator.rs`):**
      - `init`: Spawn hotkey listener thread. Pass coordinator sender/handle.
      - `handle_msg`:
        - Handle `HotkeyTriggered`, `WakeWordDetected`, `SilenceTimeout`.
        - Implement state transitions based on current state and trigger.
        - Start/Stop `AudioCaptureActor` based on state.
        - Signal UI about state changes (`AppOutput::UpdateStatus`).
  7.  **Configuration (`core/src/config.rs`, `src/settings.rs`):**
      - Add settings for activation mode (key/wake), hotkey combo, wake word model path, enable/disable wake word, silence timeout duration.
      - Update config load/save.
      - Add GTK widgets to settings UI (`src`) for these options.
  8.  **Wake Word Model Download:**
      - Provide script/instructions for downloading Porcupine (or other engine) model files.
  9.  **Documentation:**
      - Explain activation logic, state machine.
      - Document configuration for hotkeys, wake word, silence.
      - Instructions for wake word models.
  10. **Testing:**
      - Manual: Test hotkey start/stop. Test wake word start + silence stop. Test changing activation modes in settings. Verify UI status updates correctly reflect the internal state.

---

**Phase 9: Output Implementation (Typing)**

- **Goal:** Implement `OutputActor` in `core` using `enigo` to type transcriptions.
- **Tasks:**
  1.  **Dependencies (`core/Cargo.toml`):**
      - Add `enigo`.
  2.  **Output Actor (`core/src/output.rs`):**
      - Define `struct OutputActor { enigo: enigo::Enigo }`.
      - Implement `Actor`.
      - `init`: Create `Enigo::new()`.
      - `handle_msg`: Handles `FinalTranscription` message.
        - Call `self.enigo.text(&result.0)`.
        - Handle potential `enigo` errors.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - `init`: Spawn `OutputActor`. Store handle.
      - Modify `handle_msg` for `InternalTranscriptionResult`:
        - If state indicates transcription should be typed (not a command, Phase 10), `cast!` the `FinalTranscription` message to `OutputActor`.
  4.  **Documentation:**
      - Explain `OutputActor` function.
  5.  **Testing:**
      - Manual: `cargo run`. Activate recording, speak. Verify transcribed text is typed into the currently focused application window after transcription completes.

---

**Phase 10: Command Handling**

- **Goal:** Implement command definition, parsing, and execution.
- **Tasks:**
  1.  **Command Configuration (`core/src/config.rs`):**
      - Define structs for command definitions (e.g., `trigger: String`, `action: CommandAction`).
      - `enum CommandAction { Type { template: String }, Exec { program: String, args: Vec<String> } }`. Use templates like `{text}`.
      - Add `commands: HashMap<String, CommandAction>` to `Settings`.
      - Update config load/save. Use TOML tables for commands.
  2.  **Command Logic (`core/src/command.rs`, `core/src/output.rs`):**
      - Create `fn parse_and_execute(transcription: &str, config: &Settings, output_actor: &Actor<OutputActor>)`.
      - Logic:
        - Iterate through `config.commands`. Check if transcription starts with command `trigger`.
        - If match found:
          - Extract arguments/remaining text.
          - Substitute into `action` template.
          - If `Exec`: Use `std::process::Command::new(program).args(args).spawn()`. Run in a separate thread (`std::thread::spawn`) to avoid blocking actor. Log success/failure.
          - If `Type`: Construct text using template. `cast!` message with text to `OutputActor`.
        - If no command match: `cast!` original `FinalTranscription` to `OutputActor`.
  3.  **Coordinator Integration (`core/src/coordinator.rs`):**
      - Modify `handle_msg` for `InternalTranscriptionResult`: Call `command::parse_and_execute(&result.0, &self.config, &self.output_actor_handle)`.
  4.  **UI Configuration (`src/settings.rs`):**
      - Add a section to the settings UI to view/add/edit/remove commands (e.g., using `gtk::ListView` or `gtk::TreeView`).
      - Load/save command config via `core::config`.
  5.  **Documentation:**
      - Document command syntax in config file.
      - Explain available actions (`Type`, `Exec`) and templating.
      - Explain command settings UI.
  6.  **Testing:**
      - Manual: Define commands in config (e.g., "open browser", "search for {text}"). Test speaking commands. Verify correct execution (browser opens, text is typed). Test editing commands via UI.

---

**Phase 11: Error Handling & Robustness**

- **Goal:** Systematically improve error handling, logging, and recovery.
- **Tasks:**
  1.  **Error Types (`core/src/errors.rs`):**
      - Define custom error enums using `thiserror` for different modules (`AudioError`, `ConfigError`, `IpcError`, `CommandError`, etc.).
  2.  **Logging (`src/main.rs`, `core/src/lib.rs`, `transcriber/src/main.rs`):**
      - Configure `tracing_subscriber` for multiple layers: console (dev) and rotating file (user).
      - Add more detailed `span!`s and event logging throughout actors and threads. Log important state changes and errors.
  3.  **Transcriber Resilience (`core/src/transcriber_client.rs`):**
      - Monitor the `transcriber` child process handle.
      - If process exits unexpectedly:
        - Log the error.
        - Signal `AppCoordinatorActor`.
        - Implement optional auto-restart logic (with backoff).
      - Handle broken pipe errors during stdin write more gracefully.
  4.  **Actor Error Handling (`core`):**
      - Propagate errors using `Result` where appropriate within actor logic.
      - Handle errors returned by Stakker `call!` / `query!`.
      - Report critical errors from actors back to `AppCoordinatorActor`.
  5.  **User Feedback (`src/ui.rs`, `core/src/coordinator.rs`):**
      - `AppCoordinatorActor`: When receiving critical error signals, format user-friendly messages.
      - Send error messages to UI (`AppOutput::ShowError(String)`).
      - Implement `ShowError` in `AppModel::update` (`src`) to display a `gtk::MessageDialog` or `gtk::InfoBar`.
  6.  **Configuration Validation (`core/src/config.rs`):**
      - Add validation logic after loading config (e.g., check if model path exists). Report errors.
  7.  **Documentation:**
      - Describe logging setup (file location).
      - Document common errors and troubleshooting steps.
  8.  **Testing:**
      - Manual: Kill `transcriber` process, verify error message/restart. Provide invalid config, verify error. Test error dialogs. Check log files.

---

**Phase 12: Packaging & Documentation**

- **Goal:** Create distributable packages, write final documentation.
- **Tasks:**
  1.  **Build Scripts (`scripts/`):**
      - Create build/packaging scripts (e.g., `build_linux.sh`, `build_windows.bat`, `build_macos.sh`) or use a tool like `cargo-dist` if adaptable.
      - Scripts should:
        - Compile main app (`cargo build --release`).
        - Compile transcriber (`cargo build --release -p whisperkey-transcriber`).
        - Create platform-specific bundle structure (e.g., `WhisperKey.app` on macOS, directory with .exe/.dlls on Win, standard Linux structure).
        - Copy main executable, transcriber executable.
        - Copy assets (icons).
        - Include default config file.
        - Include necessary runtime libraries (GTK, potentially MSVC runtime on Win). Address `tauri-plugin-system-tray` resource needs.
        - Provide instructions/script for downloading Vosk model into the bundle or user config dir.
        - Create final archive/installer (.dmg, .zip/.msi, .deb/.tar.gz).
  2.  **Refine Icons & Assets:**
      - Create proper application icons for different platforms/sizes.
  3.  **User Documentation:**
      - Write comprehensive User Guide:
        - Installation instructions for each platform (using packages).
        - First-time setup (model download, config).
        - Detailed usage guide (UI, tray, activation).
        - Configuration reference (all settings, commands).
        - Troubleshooting.
  4.  **Developer Documentation:**
      - Finalize `README.md` (overview, quick start build).
      - Add `CONTRIBUTING.md`.
      - Generate code documentation (`cargo doc --open`). Ensure public API in `core` is well-documented.
      - Include architecture diagrams.
  5.  **Licensing:**
      - Add `LICENSE` file (e.g., MIT or GPL).
  6.  **Final Testing:**
      - Test installation and functionality _from the created packages_ on clean VMs/machines for each target platform (Win, macOS, Linux).
      - Perform regression testing of all features.
  7.  **Release:**
      - Tag release in version control.
      - Upload packages (e.g., to GitHub Releases).
