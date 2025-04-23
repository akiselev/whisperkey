use gtk::prelude::*;
use relm4::prelude::*;
use stakker::{call, Stakker};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;
// Removed explicit 'cast' import, relying on stakker/core being in scope
use whisperkey_core::{init_core_actors, CoordinatorMsg, CoreHandles};

// AppInput enum for Relm4
#[derive(Debug)]
enum AppInput {
    TestCore,
}

struct AppModel {
    core_handles: Rc<RefCell<Option<CoreHandles>>>,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppInput;
    type Output = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let core_handles_rc = Rc::new(RefCell::new(None));
        let model = Self {
            core_handles: core_handles_rc,
        };

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
                gtk::Button {
                    set_label: "Test Core",
                    connect_clicked => AppInput::TestCore,
                },
            }
        }
    }

    fn update(&mut self, input: AppInput, _sender: ComponentSender<Self>) {
        match input {
            AppInput::TestCore => {
                if let Some(handles) = &*self.core_handles.borrow() {
                    call!([handles.coordinator], handle_test());
                    println!("Sent TestCore message via call! [syntax 2]");
                } else {
                    println!("Core handles not initialized yet.");
                }
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let mut stakker = Stakker::new(Instant::now());
    let core_handles = init_core_actors(&mut stakker);
    let core_handles_rc = Rc::new(RefCell::new(Some(core_handles)));

    let app = RelmApp::new("org.example.whisperkey_phase2_test");

    let model = AppModel {
        core_handles: core_handles_rc.clone(),
    };

    app.run::<AppModel>(());

    // Note: Stakker needs to run to process actor messages.
    // If the Relm4 app exits immediately, Stakker might not run long enough.
    // Consider running stakker in a separate thread or using `stakker.run()`
    // after `app.run()` if background processing is needed after UI closes.
    // For now, `app.run()` blocks until the UI window is closed. Stakker might
    // process messages while the UI is active.

    // Example of keeping Stakker running explicitly (if needed):
    // This would block after the UI closes.
    // println!("UI closed, running Stakker...");
    // stakker.run(std::time::Duration::from_secs(5)); // Run for 5 more seconds
    println!("Finished.");
}
