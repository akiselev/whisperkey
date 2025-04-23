use gtk4::prelude::*;
use gtk4::{
    Button, CheckButton, ComboBoxText, Dialog, Entry, FileChooserAction, FileChooserDialog, Label,
    SpinButton, Window,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use whisperkey_core::{config::CommandAction, load_config, save_config, Settings, VadMode};

pub fn show_settings_dialog(parent: &Window) -> bool {
    // Load current settings
    let settings = Rc::new(RefCell::new(
        (*load_config().unwrap_or_else(|_| {
            eprintln!("Failed to load settings");
            Arc::new(Settings::default())
        }))
        .clone(),
    ));

    // Create a new dialog
    let dialog = Dialog::new();
    dialog.set_title(Some("Settings"));
    dialog.set_modal(true);
    dialog.set_default_width(400);
    dialog.set_default_height(500); // Increased height for new settings
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
    if let Some(path) = &settings.borrow().model_path {
        model_path_entry.set_text(path);
    }
    inner_box.append(&model_path_entry);

    let browse_button = Button::with_label("Browse...");
    inner_box.append(&browse_button);

    model_path_box.append(&inner_box);
    content_area.append(&model_path_box);

    // Audio Processing Section Header
    let audio_section_label = Label::new(Some("Audio Processing"));
    audio_section_label.set_halign(gtk4::Align::Start);
    audio_section_label.set_margin_top(12);
    audio_section_label.set_margin_bottom(6);
    content_area.append(&audio_section_label);

    // Separator after section header
    let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    separator.set_margin_bottom(12);
    content_area.append(&separator);

    // Noise Reduction Checkbox
    let denoise_check = CheckButton::with_label("Enable Noise Reduction");
    denoise_check.set_active(settings.borrow().enable_denoise);
    denoise_check.set_margin_bottom(6);
    content_area.append(&denoise_check);

    // VAD Checkbox
    let vad_check = CheckButton::with_label("Enable Voice Activity Detection (VAD)");
    vad_check.set_active(settings.borrow().enable_vad);
    vad_check.set_margin_bottom(6);
    content_area.append(&vad_check);

    // VAD Mode Combo Box
    let vad_mode_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    vad_mode_box.set_margin_bottom(6);
    vad_mode_box.set_margin_start(24); // Indent

    let vad_mode_label = Label::new(Some("VAD Mode:"));
    vad_mode_label.set_halign(gtk4::Align::Start);
    vad_mode_box.append(&vad_mode_label);

    let vad_mode_combo = ComboBoxText::new();
    vad_mode_combo.append(Some("quality"), "Quality");
    vad_mode_combo.append(Some("low_bitrate"), "Low Bitrate");
    vad_mode_combo.append(Some("aggressive"), "Aggressive");
    vad_mode_combo.append(Some("very_aggressive"), "Very Aggressive");

    // Set the active option based on settings
    match settings.borrow().vad_mode {
        VadMode::Quality => vad_mode_combo.set_active_id(Some("quality")),
        VadMode::LowBitrate => vad_mode_combo.set_active_id(Some("low_bitrate")),
        VadMode::Aggressive => vad_mode_combo.set_active_id(Some("aggressive")),
        VadMode::VeryAggressive => vad_mode_combo.set_active_id(Some("very_aggressive")),
    };
    vad_mode_combo.set_margin_start(6);
    vad_mode_box.append(&vad_mode_combo);
    content_area.append(&vad_mode_box);

    // Energy Threshold Slider
    let energy_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    energy_box.set_margin_bottom(6);
    energy_box.set_margin_start(24); // Indent

    let energy_label = Label::new(Some("Energy Threshold:"));
    energy_label.set_halign(gtk4::Align::Start);
    energy_box.append(&energy_label);

    // Use SpinButton for more precise control
    let energy_spin = SpinButton::with_range(0.001, 0.5, 0.001);
    energy_spin.set_value(settings.borrow().vad_energy_threshold as f64);
    energy_spin.set_digits(3); // Show 3 decimal places
    energy_spin.set_margin_start(6);
    energy_box.append(&energy_spin);

    content_area.append(&energy_box);

    // Silence Threshold
    let silence_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    silence_box.set_margin_bottom(12);
    silence_box.set_margin_start(24); // Indent

    let silence_label = Label::new(Some("Silence Threshold (ms):"));
    silence_label.set_halign(gtk4::Align::Start);
    silence_box.append(&silence_label);

    let silence_spin = SpinButton::with_range(100.0, 5000.0, 100.0);
    silence_spin.set_value(settings.borrow().silence_threshold_ms as f64);
    silence_spin.set_margin_start(6);
    silence_box.append(&silence_spin);
    content_area.append(&silence_box);

    // Keyboard Output Section Header
    let keyboard_section_label = Label::new(Some("Keyboard Output"));
    keyboard_section_label.set_halign(gtk4::Align::Start);
    keyboard_section_label.set_margin_top(12);
    keyboard_section_label.set_margin_bottom(6);
    content_area.append(&keyboard_section_label);

    // Separator after section header
    let keyboard_separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    keyboard_separator.set_margin_bottom(12);
    content_area.append(&keyboard_separator);

    // Keyboard Output Checkbox
    let keyboard_check = CheckButton::with_label("Enable Keyboard Output (simulates typing)");
    keyboard_check.set_active(settings.borrow().enable_keyboard_output);
    keyboard_check.set_margin_bottom(6);
    content_area.append(&keyboard_check);

    // Delay before typing
    let delay_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    delay_box.set_margin_bottom(12);
    delay_box.set_margin_start(24); // Indent

    let delay_label = Label::new(Some("Delay before typing (ms):"));
    delay_label.set_halign(gtk4::Align::Start);
    delay_box.append(&delay_label);

    let delay_spin = SpinButton::with_range(0.0, 5000.0, 100.0);
    delay_spin.set_value(settings.borrow().keyboard_output_delay_ms as f64);
    delay_spin.set_margin_start(6);
    delay_box.append(&delay_spin);
    content_area.append(&delay_box);

    // Warning label for keyboard output
    let warning_label = Label::new(Some("Warning: Enabling keyboard output will type the transcribed text into any active application."));
    warning_label.set_margin_start(24);
    warning_label.set_margin_bottom(12);
    warning_label.set_wrap(true);
    warning_label.set_width_chars(40);
    warning_label.add_css_class("warning");
    content_area.append(&warning_label);

    // Commands Section Header
    let commands_section_label = Label::new(Some("Voice Commands"));
    commands_section_label.set_halign(gtk4::Align::Start);
    commands_section_label.set_margin_top(12);
    commands_section_label.set_margin_bottom(6);
    content_area.append(&commands_section_label);

    // Separator after section header
    let commands_separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    commands_separator.set_margin_bottom(12);
    content_area.append(&commands_separator);

    // Simplified command management - text based
    let command_info_label =
        Label::new(Some("Voice commands are defined in the config.toml file."));
    command_info_label.set_margin_bottom(12);
    command_info_label.set_wrap(true);
    command_info_label.set_width_chars(40);
    content_area.append(&command_info_label);

    // Display existing commands in a text view
    let commands_info = format!(
        "Default commands:\n\n{}\n\nCommands can be either Type or Exec and support {{args}} substitution.",
        settings
            .borrow()
            .commands
            .iter()
            .map(|(trigger, action)| match action {
                CommandAction::Type(template) => {
                    format!("• \"{}\" → Type: {}", trigger, template)
                }
                CommandAction::Exec(template) => {
                    format!("• \"{}\" → Exec: {}", trigger, template)
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    );

    let commands_label = Label::new(Some(&commands_info));
    commands_label.set_margin_bottom(12);
    commands_label.set_margin_start(12);
    commands_label.set_wrap(true);
    commands_label.set_width_chars(40);
    commands_label.set_halign(gtk4::Align::Start);
    content_area.append(&commands_label);

    // Handle VAD checkbox change
    let vad_mode_box_clone = vad_mode_box.clone();
    let energy_box_clone = energy_box.clone();
    let silence_box_clone = silence_box.clone();
    vad_check.connect_toggled(move |check| {
        let enabled = check.is_active();
        vad_mode_box_clone.set_sensitive(enabled);
        energy_box_clone.set_sensitive(enabled);
        silence_box_clone.set_sensitive(enabled);
    });

    // Handle keyboard output checkbox change
    let delay_box_clone = delay_box.clone();
    keyboard_check.connect_toggled(move |check| {
        let enabled = check.is_active();
        delay_box_clone.set_sensitive(enabled);
    });

    // Initialize sensitivity
    vad_mode_box.set_sensitive(settings.borrow().enable_vad);
    energy_box.set_sensitive(settings.borrow().enable_vad);
    silence_box.set_sensitive(settings.borrow().enable_vad);
    delay_box.set_sensitive(settings.borrow().enable_keyboard_output);

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
    let denoise_check_for_response = denoise_check.clone();
    let vad_check_for_response = vad_check.clone();
    let vad_mode_combo_for_response = vad_mode_combo.clone();
    let energy_spin_for_response = energy_spin.clone();
    let silence_spin_for_response = silence_spin.clone();
    let keyboard_check_for_response = keyboard_check.clone();
    let delay_spin_for_response = delay_spin.clone();
    let settings_clone = settings.clone();

    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            // Create a new settings with the updated values
            let mut new_settings = settings_clone.borrow().clone();

            // Model path
            let model_path_text = model_path_entry_for_response.text().to_string();
            new_settings.model_path = if model_path_text.is_empty() {
                None
            } else {
                Some(model_path_text)
            };

            // Audio processing settings
            new_settings.enable_denoise = denoise_check_for_response.is_active();
            new_settings.enable_vad = vad_check_for_response.is_active();

            // VAD mode
            new_settings.vad_mode = match vad_mode_combo_for_response.active_id().as_deref() {
                Some("quality") => VadMode::Quality,
                Some("low_bitrate") => VadMode::LowBitrate,
                Some("aggressive") => VadMode::Aggressive,
                Some("very_aggressive") => VadMode::VeryAggressive,
                _ => VadMode::Quality, // Default
            };

            // Energy threshold
            new_settings.vad_energy_threshold = energy_spin_for_response.value() as f32;

            // Silence threshold
            new_settings.silence_threshold_ms = silence_spin_for_response.value() as u32;

            // Keyboard output settings
            new_settings.enable_keyboard_output = keyboard_check_for_response.is_active();
            new_settings.keyboard_output_delay_ms = delay_spin_for_response.value() as u32;

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
