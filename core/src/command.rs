use regex::Regex;
use std::collections::HashMap;
use std::process::Command as ProcessCommand;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::config::CommandAction;
use crate::types::KeyboardOutputMsg;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Failed to execute command: {0}")]
    ExecutionError(String),

    #[error("Command parse error: {0}")]
    ParseError(String),
}

/// Process a transcription to check for commands.
/// Returns Some(()) if a command was executed, or None if the text should be typed normally.
pub fn process_command(
    text: &str,
    commands: &HashMap<String, CommandAction>,
    keyboard_output_fn: impl Fn(KeyboardOutputMsg) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Option<()>, CommandError> {
    // Check if the text matches any command trigger
    for (trigger, action) in commands {
        // Escape special regex characters in the trigger pattern
        let pattern = format!(r"(?i)^\s*{}\s*(.*)$", regex::escape(trigger));

        // Parse the regex and check if it matches
        match Regex::new(&pattern) {
            Ok(regex) => {
                if let Some(captures) = regex.captures(text) {
                    // Extract arguments from the capture group
                    let args = captures.get(1).map_or("", |m| m.as_str()).trim();

                    info!(
                        "Command trigger '{}' matched with args: '{}'",
                        trigger, args
                    );

                    // Execute the command action
                    match action {
                        CommandAction::Type(template) => {
                            // Substitute args into template
                            let output_text = template.replace("{args}", args);
                            debug!("Typing: {}", output_text);

                            // Send to keyboard output
                            keyboard_output_fn(KeyboardOutputMsg::TypeText(output_text))
                                .map_err(|e| CommandError::ExecutionError(e.to_string()))?;
                        }
                        CommandAction::Exec(template) => {
                            // Substitute args into template
                            let command_str = template.replace("{args}", args);
                            debug!("Executing: {}", command_str);

                            // Split the command string into program and arguments
                            let parts: Vec<&str> = command_str.split_whitespace().collect();
                            if parts.is_empty() {
                                return Err(CommandError::ExecutionError(
                                    "Empty command".to_string(),
                                ));
                            }

                            // Spawn a thread to execute the command
                            let program = parts[0].to_string();
                            let args: Vec<String> =
                                parts[1..].iter().map(|s| s.to_string()).collect();

                            std::thread::spawn(move || {
                                match ProcessCommand::new(&program).args(&args).spawn() {
                                    Ok(mut child) => match child.wait() {
                                        Ok(status) => debug!("Command exited with: {}", status),
                                        Err(e) => warn!("Failed to wait for command: {}", e),
                                    },
                                    Err(e) => warn!("Failed to execute command: {}", e),
                                }
                            });
                        }
                    }

                    // Return that we handled a command
                    return Ok(Some(()));
                }
            }
            Err(e) => {
                warn!("Invalid regex pattern for trigger '{}': {}", trigger, e);
                continue;
            }
        }
    }

    // No command matched
    Ok(None)
}
