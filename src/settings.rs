use gtk4::prelude::*;
use gtk4::{Button, Dialog, Entry, FileChooserAction, FileChooserDialog, Label, Window};
use std::sync::Arc;

use whisperkey_core::{load_config, save_config, Settings};

pub fn show_settings_dialog(parent: &Window) -> bool {
    // Load current settings
    let settings = load_config().unwrap_or_else(|_| {
        eprintln!("Failed to load settings");
        Arc::new(Settings::default())
    });

    // Create a new dialog
    let dialog = Dialog::new();
    dialog.set_title(Some("Settings"));
    dialog.set_modal(true);
    dialog.set_default_width(400);
    dialog.set_default_height(300);
    dialog.set_transient_for(Some(parent));

    // Add cancel and save buttons
    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
    dialog.add_button("Save", gtk4::ResponseType::Accept);

    // Create content box
    let content_area = dialog.content_area();
    content_area.set_orientation(gtk4::Orientation::Vertical);
    content_area.set_spacing(12);
    content_area.set_margin_top(12);
    content_area.set_margin_bottom(12);
    content_area.set_margin_start(12);
    content_area.set_margin_end(12);

    // Model path row
    let model_path_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    model_path_box.set_margin_bottom(12);

    let model_path_label = Label::new(Some("Vosk Model Path:"));
    model_path_label.set_halign(gtk4::Align::Start);
    model_path_label.set_valign(gtk4::Align::Center);
    model_path_box.append(&model_path_label);

    let inner_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    inner_box.set_hexpand(true);

    let model_path_entry = Entry::new();
    model_path_entry.set_hexpand(true);
    model_path_entry.set_placeholder_text(Some("Path to Vosk model directory"));
    if let Some(path) = &settings.model_path {
        model_path_entry.set_text(path);
    }
    inner_box.append(&model_path_entry);

    let browse_button = Button::with_label("Browse...");
    inner_box.append(&browse_button);

    model_path_box.append(&inner_box);
    content_area.append(&model_path_box);

    // Handle browse button click
    let model_path_entry_clone = model_path_entry.clone();
    let dialog_clone = dialog.clone();
    browse_button.connect_clicked(move |_| {
        let file_chooser = FileChooserDialog::new(
            Some("Select Vosk Model Directory"),
            Some(&dialog_clone), // Set transient for the settings dialog
            FileChooserAction::SelectFolder,
            &[
                ("Cancel", gtk4::ResponseType::Cancel),
                ("Select", gtk4::ResponseType::Accept),
            ],
        );

        // Make sure the file chooser is modal
        file_chooser.set_modal(true);

        let model_path_entry_inner = model_path_entry_clone.clone();
        file_chooser.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        if let Some(path_str) = path.to_str() {
                            model_path_entry_inner.set_text(path_str);
                        }
                    }
                }
            }
            dialog.destroy();
        });

        file_chooser.show();
    });

    // Connect the response signal
    let model_path_entry_for_response = model_path_entry.clone();
    let settings_clone = settings.clone();

    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            // Create a new settings with the updated values
            let mut new_settings = (*settings_clone).clone();
            let model_path_text = model_path_entry_for_response.text().to_string();
            new_settings.model_path = if model_path_text.is_empty() {
                None
            } else {
                Some(model_path_text)
            };

            // Save the settings
            if let Err(e) = save_config(&new_settings) {
                eprintln!("Failed to save settings: {}", e);
            } else {
                println!("Settings saved successfully");
            }
        }

        dialog.destroy();
    });

    // Show the dialog and return immediately
    dialog.show();

    // Return true to indicate the dialog was shown
    true
}
