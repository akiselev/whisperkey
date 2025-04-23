use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use whisperkey_core::{init_core_actors, types::AppOutput, CoordinatorMsg, CoreHandles};

// AppInput enum for Relm4
#[derive(Debug)]
enum AppInput {
    TestCore,
    StartListening,
    StopListening,
    ProcessOutput(AppOutput),
    UpdateCoreHandles,
}

struct AppModel {
    core_handles: Option<CoreHandles>,
    status_text: String,
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
        let model = Self {
            core_handles: None,
            status_text: "Starting...".to_string(),
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

            // Initialize core actors
            let core_handles = init_core_actors(ui_sender).await;

            // Store core handles in a thread-local static to pass back to the model
            thread_local! {
                static CORE_HANDLES: std::cell::RefCell<Option<CoreHandles>> = std::cell::RefCell::new(None);
            }

            CORE_HANDLES.with(|h| {
                *h.borrow_mut() = Some(core_handles);
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
            set_title: Some("WhisperKey Phase 2 Test Shell"),
            set_default_width: 400,
            set_default_height: 200,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 24,
                gtk::Label {
                    #[watch]
                    set_label: &model.status_text,
                },
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
            AppInput::ProcessOutput(output) => {
                match output {
                    AppOutput::UpdateStatus(status) => {
                        self.status_text = status;
                        println!("Status updated: {}", self.status_text);
                    } // Handle other output types as they're added
                }
            }
            AppInput::UpdateCoreHandles => {
                // Get the core handles from the thread-local
                thread_local! {
                    static CORE_HANDLES: std::cell::RefCell<Option<CoreHandles>> = std::cell::RefCell::new(None);
                }

                CORE_HANDLES.with(|h| {
                    if let Some(handles) = h.borrow_mut().take() {
                        self.core_handles = Some(handles);
                        println!("Core handles updated");
                    }
                });
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let app = RelmApp::new("org.example.whisperkey_phase2_test");
    app.run::<AppModel>(());

    println!("Finished.");
}
