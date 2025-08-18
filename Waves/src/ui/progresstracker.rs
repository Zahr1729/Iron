use std::sync::mpsc;

use eframe::egui::{self, Widget};

pub struct ProgressTracker {
    progress: f32,
    pub tx: mpsc::Sender<f32>,
    rx: mpsc::Receiver<f32>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            progress: 0.0,
            tx,
            rx,
        }
    }
}

impl Widget for &mut ProgressTracker {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        while let Ok(p) = self.rx.try_recv() {
            self.progress = p;
        }

        if self.progress > 0.0 {
            ui.add(egui::ProgressBar::new(self.progress).desired_width(100.0))
        } else {
            ui.response()
        }
    }
}
