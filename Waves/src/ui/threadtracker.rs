use std::{mem, thread::JoinHandle};

use eframe::egui::{self, Response, Widget};
use symphonia::core::errors::Error;

use crate::ui::progresstracker::ProgressTracker;

pub struct ThreadTracker {
    prog_tracker: ProgressTracker,
    handle: Option<JoinHandle<Result<(), Error>>>,
    thread_name: String,
    pub should_dismiss: bool,
    output_message: Option<String>,
}

impl ThreadTracker {
    pub fn new(
        prog_tracker: ProgressTracker,
        handle: JoinHandle<Result<(), Error>>,
        thread_name: String,
    ) -> Self {
        Self {
            prog_tracker,
            handle: Some(handle),
            thread_name,
            should_dismiss: false,
            output_message: None,
        }
    }

    pub fn check_is_done(&mut self) {
        match &mut self.handle {
            None => return (),
            Some(h) => {
                if h.is_finished() {
                    ()
                } else {
                    return ();
                }
            }
        };
        // we know h is some and h.is_finished

        // this wizardry allows us to steal the data in self.handle and put into h by simply swapping the memory
        let mut h = None;
        mem::swap(&mut h, &mut self.handle);

        match h.unwrap().join().unwrap() {
            Ok(()) => {
                self.output_message = Some("Done".to_string());
                ()
            }
            Err(e) => {
                self.output_message =
                    Some(format!("Error: {:?} in thread {}", e, self.thread_name));
                ()
            }
        }
    }
}

impl Widget for &mut ThreadTracker {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let mut r = ui.add(egui::Label::new(self.thread_name.clone()));
        r = ui.add(&mut self.prog_tracker).union(r);

        if let Some(message) = &self.output_message {
            r = r.on_hover_text(message);

            if r.clicked() {
                self.should_dismiss = true;
            }
        } else {
            r = r.on_hover_text("In Progress");
        }
        r
    }
}
