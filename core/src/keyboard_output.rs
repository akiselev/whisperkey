use enigo::{Enigo, Keyboard, Settings};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::config::Settings as AppSettings;
use crate::types::{CoordinatorMsg, KeyboardOutputMsg};

// Error types for keyboard output
#[derive(Error, Debug)]
pub enum KeyboardOutputError {
    #[error("Failed to initialize keyboard controller: {0}")]
    InitError(String),

    #[error("Failed to type text: {0}")]
    TypeError(String),
}

pub struct KeyboardOutputActor {}

pub struct KeyboardOutputState {
    enigo: Enigo,
    coordinator: ActorRef<CoordinatorMsg>,
    config: Arc<AppSettings>,
    enabled: bool,
}

#[ractor::async_trait]
impl Actor for KeyboardOutputActor {
    type Msg = KeyboardOutputMsg;
    type State = KeyboardOutputState;
    type Arguments = (ActorRef<CoordinatorMsg>, Arc<AppSettings>);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (coordinator, config): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("KeyboardOutputActor started");

        // Initialize Enigo keyboard controller
        // Create default settings for Enigo
        let enigo_settings = Settings::default();
        let enigo = match Enigo::new(&enigo_settings) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to initialize keyboard controller: {:?}", e);
                return Err(ActorProcessingErr::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to initialize keyboard controller: {:?}", e),
                )));
            }
        };

        // Create state with default values
        let state = KeyboardOutputState {
            enigo,
            coordinator,
            config,
            enabled: false, // Disabled by default for safety
        };

        // Send status update to coordinator
        state
            .coordinator
            .send_message(CoordinatorMsg::UpdateStatus(
                "Keyboard output initialized (disabled by default)".to_string(),
            ))
            .ok();

        Ok(state)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            KeyboardOutputMsg::TypeText(text) => {
                if state.enabled {
                    tracing::info!("Typing text: {}", text);

                    // Small delay to let user prepare
                    if state.config.keyboard_output_delay_ms > 0 {
                        // Use std::thread::sleep instead of tokio::time::sleep
                        std::thread::sleep(Duration::from_millis(
                            state.config.keyboard_output_delay_ms as u64,
                        ));
                    }

                    // Type the text
                    if let Err(e) = state.enigo.text(&text) {
                        tracing::error!("Failed to type text: {:?}", e);
                    }

                    // Notify coordinator of success
                    state
                        .coordinator
                        .send_message(CoordinatorMsg::UpdateStatus(format!(
                            "Typed text: {}",
                            text
                        )))
                        .ok();
                } else {
                    tracing::info!("Keyboard output is disabled, not typing: {}", text);
                    state
                        .coordinator
                        .send_message(CoordinatorMsg::UpdateStatus(
                            "Keyboard output is disabled (enable in settings)".to_string(),
                        ))
                        .ok();
                }
            }
            KeyboardOutputMsg::Enable(enable) => {
                state.enabled = enable;
                let status = if enable {
                    "Keyboard output enabled"
                } else {
                    "Keyboard output disabled"
                };
                tracing::info!("{}", status);
                state
                    .coordinator
                    .send_message(CoordinatorMsg::UpdateStatus(status.to_string()))
                    .ok();
            }
            KeyboardOutputMsg::Shutdown => {
                tracing::info!("KeyboardOutputActor shutting down");
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        tracing::info!("KeyboardOutputActor stopped");
        Ok(())
    }
}
