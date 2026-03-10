mod ai;
mod app;
mod db;

use app::ThoughtTrain;

fn main() -> iced::Result {
    let icon = load_window_icon();

    let mut app = iced::application(ThoughtTrain::title, ThoughtTrain::update, ThoughtTrain::view)
        .theme(ThoughtTrain::theme)
        .window_size((750.0, 580.0));

    if let Some(icon) = icon {
        app = app.window(iced::window::Settings {
            icon: Some(icon),
            ..Default::default()
        });
    }

    app.run_with(ThoughtTrain::new)
}

fn load_window_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../logo.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    iced::window::icon::from_rgba(img.into_raw(), w, h).ok()
}
