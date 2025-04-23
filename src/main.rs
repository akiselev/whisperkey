use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use whisperkey_core::{
    init_core_actors, load_config, types::AppOutput, CoordinatorMsg, CoreHandles,
};

mod settings;

// AppInput enum for Relm4
#[derive(Debug)]
enum AppInput {
    TestCore,
    StartListening,
    StopListening,
    ProcessOutput(AppOutput),
    UpdateCoreHandles,
    UpdateTextBuffer(String),
    OpenSettings,
    ToggleKeyboardOutput(bool),
}

struct AppModel {
    core_handles: Option<CoreHandles>,
    status_text: String,
    transcription_text: String,
    keyboard_output_enabled: bool,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppInput;
    type Output = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Load config to check for keyboard output setting
        let config = load_config().unwrap_or_else(|e| {
            eprintln!("Failed to load config: {}", e);
            Arc::new(whisperkey_core::Settings::default())
        });

        let model = Self {
            core_handles: None,
            status_text: "Starting...".to_string(),
            transcription_text: "".to_string(),
            keyboard_output_enabled: config.enable_keyboard_output,
        };

        // Setup a background worker to receive messages from the core
        let sender_clone = sender.clone();
        relm4::spawn_local(async move {
            let (tx, mut rx) = mpsc::channel::<AppOutput>(100);

            // Create UI sender function used by the core
            let tx_clone = tx.clone();
            let ui_sender = Arc::new(move |output: AppOutput| {
                let tx = tx_clone.clone();
                tokio::spawn(async move {
                    tx.send(output).await.unwrap_or_else(|err| {
                        eprintln!("Failed to send AppOutput to UI: {}", err);
                    });
                });
            });

            // Load config to get model path
            let config = load_config().unwrap_or_else(|e| {
                eprintln!("Failed to load config: {}", e);
                Arc::new(whisperkey_core::Settings::default())
            });

            // Use model path from config, or fall back to default
            let model_path = config
                .model_path
                .as_ref()
                .map(|p| PathBuf::from(p))
                .or_else(|| get_default_model_path());

            if let Some(path) = &model_path {
                println!("Using Vosk model at: {:?}", path);
            } else {
                println!("No model path found, transcriber will fail to start!");
            }

            // Initialize core actors
            let core_handles = init_core_actors(ui_sender, model_path).await;

            // Store core handles in a thread-local static to pass back to the model
            thread_local! {
                static CORE_HANDLES: std::cell::RefCell<Option<CoreHandles>> = std::cell::RefCell::new(None);
            }

            CORE_HANDLES.with(|h| {
                *h.borrow_mut() = Some(core_handles.expect("Failed to initialize core actors"));
            });

            // Signal to update core handles and status
            sender_clone.input(AppInput::UpdateCoreHandles);
            sender_clone.input(AppInput::ProcessOutput(AppOutput::UpdateStatus(
                "Core initialized".to_string(),
            )));

            // Process messages from the core and forward to the UI
            while let Some(output) = rx.recv().await {
                sender_clone.input(AppInput::ProcessOutput(output));
            }
        });

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            set_title: Some("WhisperKey"),
            set_default_width: 600,
            set_default_height: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,

                // Menu bar
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 0,
                    set_margin_all: 0,

                    gtk::MenuButton {
                        set_label: "Menu",

                        #[wrap(Some)]
                        set_popover = &gtk::Popover {
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 4,
                                set_spacing: 4,

                                gtk::Button {
                                    set_label: "Settings",
                                    connect_clicked[sender] => move |_| {
                                        sender.input(AppInput::OpenSettings);
                                    }
                                },
                            }
                        }
                    }
                },

                // Main content
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 24,
                    set_vexpand: true,

                    // Status area
                    gtk::Label {
                        #[watch]
                        set_label: &model.status_text,
                        set_margin_bottom: 12,
                    },

                    // Control buttons
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_margin_bottom: 20,

                        gtk::Button {
                            set_label: "Test Core",
                            connect_clicked => AppInput::TestCore,
                        },
                        gtk::Button {
                            set_label: "Start Listening",
                            connect_clicked => AppInput::StartListening,
                        },
                        gtk::Button {
                            set_label: "Stop Listening",
                            connect_clicked => AppInput::StopListening,
                        },
                    },

                    // Keyboard output toggle
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_margin_bottom: 12,

                        gtk::CheckButton {
                            set_label: Some("Enable Keyboard Output"),
                            #[watch]
                            set_active: model.keyboard_output_enabled,
                            connect_toggled[sender] => move |btn| {
                                let active = btn.is_active();
                                sender.input(AppInput::ToggleKeyboardOutput(active));
                            }
                        },

                        gtk::Label {
                            set_text: "Warning: Will type text into the active application!",
                            set_margin_start: 10,
                        }
                    },

                    // Transcription area
                    gtk::Label {
                        set_label: "Transcription Results:",
                        set_halign: gtk::Align::Start,
                        set_margin_bottom: 6,
                    },

                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_min_content_height: 200,

                        gtk::TextView {
                            set_editable: false,
                            set_cursor_visible: false,
                            set_wrap_mode: gtk::WrapMode::Word,

                            // Update text when transcription changes
                            #[watch]
                            set_buffer: Some(&{
                                let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
                                buffer.set_text(&model.transcription_text);
                                buffer
                            }),
                        },
                    },
                }
            }
        }
    }

    fn update(&mut self, input: AppInput, _sender: ComponentSender<Self>) {
        match input {
            AppInput::TestCore => {
                if let Some(handles) = &self.core_handles {
                    handles
                        .coordinator
                        .send_message(CoordinatorMsg::HandleTest)
                        .unwrap();
                    println!("Sent TestCore message to coordinator");
                } else {
                    println!("Core handles not initialized yet.");
                    self.status_text = "Core not ready yet".to_string();
                }
            }
            AppInput::StartListening => {
                if let Some(handles) = &self.core_handles {
                    handles
                        .coordinator
                        .send_message(CoordinatorMsg::StartListening)
                        .unwrap();
                    println!("Sent StartListening message to coordinator");
                } else {
                    self.status_text = "Core not ready yet".to_string();
                }
            }
            AppInput::StopListening => {
                if let Some(handles) = &self.core_handles {
                    handles
                        .coordinator
                        .send_message(CoordinatorMsg::StopListening)
                        .unwrap();
                    println!("Sent StopListening message to coordinator");
                } else {
                    self.status_text = "Core not ready yet".to_string();
                }
            }
            AppInput::ToggleKeyboardOutput(enable) => {
                if let Some(handles) = &self.core_handles {
                    handles
                        .coordinator
                        .send_message(CoordinatorMsg::ToggleKeyboardOutput(enable))
                        .unwrap();
                    println!("Toggled keyboard output to: {}", enable);
                    self.keyboard_output_enabled = enable;
                } else {
                    self.status_text = "Core not ready yet".to_string();
                }
            }
            AppInput::ProcessOutput(output) => match output {
                AppOutput::UpdateStatus(status) => {
                    self.status_text = status;
                }
                AppOutput::UpdateTranscription(text) => {
                    if !text.is_empty() {
                        // Append to transcription text with a newline if not empty
                        if !self.transcription_text.is_empty() {
                            self.transcription_text.push_str("\n");
                        }
                        self.transcription_text.push_str(&text);
                    }
                }
            },
            AppInput::UpdateCoreHandles => {
                // Get core handles from thread-local storage
                thread_local! {
                    static CORE_HANDLES: std::cell::RefCell<Option<CoreHandles>> = std::cell::RefCell::new(None);
                }

                CORE_HANDLES.with(|h| {
                    self.core_handles = h.borrow_mut().take();
                });
            }
            AppInput::UpdateTextBuffer(_) => {
                // Do nothing; the text is bound in the view macro
            }
            AppInput::OpenSettings => {
                // Find the parent window from the list of toplevel windows
                if let Some(window) = gtk::Window::list_toplevels().first() {
                    if let Ok(parent) = window.clone().downcast::<gtk::Window>() {
                        // Show settings dialog with the parent window
                        if settings::show_settings_dialog(&parent) {
                            // The dialog is now non-blocking, so we'll just show a message
                            // that the user may need to restart after changing settings
                            self.status_text = "Settings dialog opened. You may need to restart the app after changing settings.".to_string();
                        }
                    }
                }
            }
        }
    }
}

// Helper function to find a Vosk model
fn get_default_model_path() -> Option<PathBuf> {
    // Check a few common locations
    let possible_paths = vec![
        // Current directory
        Some(PathBuf::from("model")),
        // User's home directory
        dirs::home_dir().map(|p| p.join("vosk-model")),
        // Windows specific
        Some(PathBuf::from("C:/Program Files/Vosk/model")),
        // Linux specific
        Some(PathBuf::from("/usr/share/vosk-model")),
        // MacOS specific
        dirs::home_dir().map(|p| p.join("Library/Application Support/Vosk/model")),
    ];

    // Return the first path that exists and is a directory
    for maybe_path in possible_paths {
        if let Some(path) = maybe_path {
            if path.exists() && path.is_dir() {
                return Some(path);
            }
        }
    }

    None
}

fn main() {
    tracing_subscriber::fmt::init();

    let app = RelmApp::new("org.example.whisperkey_phase4_test");
    app.run::<AppModel>(());

    println!("Finished.");
}
