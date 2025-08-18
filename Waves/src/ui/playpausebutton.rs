use eframe::egui::{self, Response, Widget};

pub struct PlayPauseButton {
    is_paused: bool,
    play_text: String,
    pause_text: String,
}

impl PlayPauseButton {
    pub fn new(is_paused: bool) -> Self {
        Self {
            is_paused,
            play_text: "play".to_string(),
            pause_text: "pause".to_string(),
        }
    }
}

impl Widget for PlayPauseButton {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let button = match self.is_paused {
            true => ui.button(self.play_text),
            false => ui.button(self.pause_text),
        };

        button
    }
}
