mod ai;
mod db;
mod ui;

use libadwaita as adw;
use adw::prelude::*;

fn main() {
    let app = adw::Application::builder()
        .application_id("dev.thought-train.app")
        .flags(gtk4::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // On activate (first launch or re-activation), build/present the UI
    app.connect_activate(|app| {
        ui::build_ui(app);
    });

    // Handle command-line so second instance just activates the first
    app.connect_command_line(|app, _| {
        app.activate();
        0.into()
    });

    app.run();
}
