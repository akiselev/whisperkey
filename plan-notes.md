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


# Phase 2 Implementation Notes (Stakker & Core Initialization)

## 1. Stakker Setup (`core/`)
- Added `stakker` and `tracing` as dependencies in `core/Cargo.toml`.
- Implemented `AppCoordinator` actor with a `CoordinatorMsg` enum and `handle` method for logging receipt of `HandleTest`.
- Added `CoreHandles` struct and `init_core_actors` function for spawning the coordinator actor.

## 2. Stakker Integration (`src/`)
- Uncommented/added `relm4` and `gtk4` dependencies in root `Cargo.toml`.
- Refactored `src/main.rs` to:
  - Initialize Stakker and core actors.
  - Store `CoreHandles` in the app model.
  - Launch a minimal Relm4 GTK window.

## 3. UI→Core Communication
- Added a "Test Core" button to the GTK window.
- Defined `AppInput::TestCore` in the UI.
- On button click, `AppInput::TestCore` is sent to the app model, which calls `cast!` to send `CoordinatorMsg::HandleTest` to the coordinator.
- Coordinator logs receipt of the message via `tracing`.

## 4. Notes
- The previous audio/transcription logic in `main.rs` was replaced with the minimal Phase 2 UI/actor shell for strict plan compliance.
- The GTK/Relm4 shell is now functional and integrated with the actor system.
- Next: Implement audio capture and pipeline in Phase 3.
