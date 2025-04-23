use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    audio_capture::AudioCaptureActor,
    audio_processor::AudioProcessorActor,
    command,
    config::{self, Settings},
    keyboard_output::KeyboardOutputActor,
    transcriber::TranscriberActor,
    types::{
        AppOutput, AudioCaptureMsg, AudioProcessorMsg, CoordinatorMsg, KeyboardOutputMsg,
        TranscriberMsg,
    },
};

pub struct Coordinator {
    // Empty struct, state is in CoordinatorState
}

pub struct CoordinatorState {
    ui_sender: Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
    audio_capture: Option<ActorRef<AudioCaptureMsg>>,
    audio_processor: Option<ActorRef<AudioProcessorMsg>>,
    transcriber: Option<ActorRef<TranscriberMsg>>,
    keyboard_output: Option<ActorRef<KeyboardOutputMsg>>,
    sample_rate: u32,      // Add sample rate for transcriber
    config: Arc<Settings>, // Configuration loaded from file
}

#[ractor::async_trait]
impl Actor for Coordinator {
    type Msg = CoordinatorMsg;
    type State = CoordinatorState;
    type Arguments = (
        Arc<dyn Fn(AppOutput) + Send + Sync + 'static>,
        Option<PathBuf>,
    );

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (ui_sender, model_path_override): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("Coordinator actor started");

        // Load configuration
        let config = config::load_config().unwrap_or_else(|e| {
            tracing::error!("Failed to load config: {}", e);
            Arc::new(Settings::default())
        });

        // Use model path from override (CLI) or config
        let model_path = model_path_override
            .or_else(|| config.model_path.as_ref().map(|path| PathBuf::from(path)));

        // Log model path if provided
        if let Some(path) = &model_path {
            tracing::info!("Using Vosk model at: {:?}", path);
        } else {
            tracing::warn!("No model path provided, using default");
        }

        // Use a fixed sample rate for now - could be configurable later
        let sample_rate = 16000; // 16 kHz is common for speech recognition

        // Spawn the audio capture actor
        let (audio_capture, _) = Actor::spawn(None, AudioCaptureActor {}, myself.clone())
            .await
            .map_err(|e| {
                ActorProcessingErr::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to start audio capture actor: {}", e),
                ))
            })?;

        // Spawn the transcriber actor with model path
        let (transcriber, _) = Actor::spawn(
            None,
            TranscriberActor {},
            (myself.clone(), sample_rate, model_path),
        )
        .await
        .map_err(|e| {
            ActorProcessingErr::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to start transcriber actor: {}", e),
            ))
        })?;

        // Spawn the audio processor actor
        let (audio_processor, _) = Actor::spawn(
            None,
            AudioProcessorActor {},
            (
                myself.clone(),
                transcriber.clone(),
                sample_rate,
                config.clone(),
            ),
        )
        .await
        .map_err(|e| {
            ActorProcessingErr::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to start audio processor actor: {}", e),
            ))
        })?;

        // Spawn the keyboard output actor
        let (keyboard_output, _) = Actor::spawn(
            None,
            KeyboardOutputActor {},
            (myself.clone(), config.clone()),
        )
        .await
        .map_err(|e| {
            ActorProcessingErr::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to start keyboard output actor: {}", e),
            ))
        })?;

        // Send initial status to UI
        (ui_sender)(AppOutput::UpdateStatus("Initialized".to_string()));

        // Return initial state
        Ok(CoordinatorState {
            ui_sender,
            audio_capture: Some(audio_capture),
            audio_processor: Some(audio_processor),
            transcriber: Some(transcriber),
            keyboard_output: Some(keyboard_output),
            sample_rate,
            config,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CoordinatorMsg::HandleTest => {
                tracing::info!("Coordinator received HandleTest message");
                (state.ui_sender)(AppOutput::UpdateStatus("Test received".to_string()));
            }
            CoordinatorMsg::StartListening => {
                tracing::info!("Coordinator: StartListening received");
                if let Some(audio_capture) = &state.audio_capture {
                    audio_capture.send_message(AudioCaptureMsg::Start)?;
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Starting audio capture...".to_string(),
                    ));
                } else {
                    tracing::error!("Audio capture actor not available");
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Error: Audio capture not available".to_string(),
                    ));
                }
            }
            CoordinatorMsg::StopListening => {
                tracing::info!("Coordinator: StopListening received");
                if let Some(audio_capture) = &state.audio_capture {
                    audio_capture.send_message(AudioCaptureMsg::Stop)?;
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Stopping audio capture...".to_string(),
                    ));
                } else {
                    tracing::error!("Audio capture actor not available");
                    (state.ui_sender)(AppOutput::UpdateStatus(
                        "Error: Audio capture not available".to_string(),
                    ));
                }
            }
            CoordinatorMsg::AudioChunk(chunk) => {
                // Log less frequently to avoid flooding
                if chunk.0.len() % 1000 == 0 {
                    tracing::debug!("Coordinator: Received audio chunk, size: {}", chunk.0.len());
                }

                // Forward to audio processor if available
                if let Some(audio_processor) = &state.audio_processor {
                    audio_processor.send_message(AudioProcessorMsg::ProcessChunk(chunk))?;
                } else if let Some(transcriber) = &state.transcriber {
                    // Fallback if audio processor is not available
                    transcriber.send_message(TranscriberMsg::ProcessAudioChunk(chunk))?;
                }
            }
            CoordinatorMsg::UpdateStatus(status) => {
                // Forward status updates from actors to the UI
                (state.ui_sender)(AppOutput::UpdateStatus(status));
            }
            CoordinatorMsg::TranscriptionResult(transcription) => {
                tracing::info!("Received transcription result: {}", transcription.0);

                // Forward to UI
                (state.ui_sender)(AppOutput::UpdateTranscription(transcription.0.clone()));

                // Process for commands
                if let Some(keyboard_output) = &state.keyboard_output {
                    // Create a function to send messages to the keyboard output actor
                    let keyboard_sender =
                        |msg: KeyboardOutputMsg| -> Result<(), Box<dyn std::error::Error>> {
                            keyboard_output
                                .send_message(msg)
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                        };

                    // Check if transcription matches a command
                    match command::process_command(
                        &transcription.0,
                        &state.config.commands,
                        keyboard_sender,
                    ) {
                        Ok(Some(_)) => {
                            // Command was executed, do nothing further
                            tracing::info!("Command was executed");
                        }
                        Ok(None) => {
                            // No command matched, type the text if keyboard output is enabled
                            if state.config.enable_keyboard_output {
                                keyboard_output
                                    .send_message(KeyboardOutputMsg::TypeText(transcription.0))?;
                            }
                        }
                        Err(e) => {
                            // Error executing command
                            tracing::error!("Error executing command: {}", e);
                            (state.ui_sender)(AppOutput::UpdateStatus(format!(
                                "Error executing command: {}",
                                e
                            )));
                        }
                    }
                }
            }
            CoordinatorMsg::SilenceDetected(is_silence) => {
                tracing::info!("Silence state changed: {}", is_silence);
                if is_silence {
                    (state.ui_sender)(AppOutput::UpdateStatus("Silence detected".to_string()));
                } else {
                    (state.ui_sender)(AppOutput::UpdateStatus("Voice detected".to_string()));
                }
            }
            CoordinatorMsg::ToggleKeyboardOutput(enable) => {
                if let Some(keyboard_output) = &state.keyboard_output {
                    keyboard_output.send_message(KeyboardOutputMsg::Enable(enable))?;
                }
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Shutdown audio processor if it's running
        if let Some(audio_processor) = &state.audio_processor {
            let _ = audio_processor.send_message(AudioProcessorMsg::Shutdown);
        }

        // Shutdown transcriber if it's running
        if let Some(transcriber) = &state.transcriber {
            let _ = transcriber.send_message(TranscriberMsg::Shutdown);
        }

        // Shutdown keyboard output if it's running
        if let Some(keyboard_output) = &state.keyboard_output {
            let _ = keyboard_output.send_message(KeyboardOutputMsg::Shutdown);
        }

        tracing::info!("Coordinator stopped");
        Ok(())
    }
}
