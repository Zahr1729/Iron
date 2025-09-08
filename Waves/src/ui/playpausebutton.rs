use eframe::egui::{self, Image, Response, Widget};

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
            true => {
                let play_button =
                    Image::new(egui::include_image!(r"../..\svg\play-svgrepo-com.svg"));
                ui.button(play_button)
            }
            false => {
                let pause_button =
                    Image::new(egui::include_image!("../../svg/pause-svgrepo-com.svg"));
                ui.button(pause_button)
            }
        };

        button
    }
}
