use eframe::{
    egui::{self, Response, Widget},
    glow::XOR,
};
use symphonia::core::errors::Error;

use std::{
    mem,
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
};

use crate::common::Track;

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

pub struct WaveformWidget<'a> {
    track: &'a Track,
    vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
}

impl<'a> WaveformWidget<'a> {
    pub fn new(track: &'a Track) -> Self {
        Self {
            track,
            vertical: true,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
        }
    }
}

fn get_extreme(chunk: &[f32]) -> f32 {
    let mut maxvalue = 0.0f32;

    if chunk.len() < 8 {
        for &d in chunk {
            if d.abs() > maxvalue.abs() {
                maxvalue = d;
            }
        }
    } else {
        for o in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let i = (o * chunk.len() as f32) as usize;

            if chunk[i].abs() > maxvalue.abs() {
                maxvalue = chunk[i];
            }
        }
    }

    maxvalue
}

impl Widget for WaveformWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let plot_id = ui.id();

        // Initialise data eg getting start stop times and step size
        let range = if let Some(plot_memory) = egui_plot::PlotMemory::load(ui.ctx(), plot_id) {
            plot_memory.bounds().range_x()
        } else {
            0.02..=1000.0
        };

        let time_span = range.end() - range.start();
        let samp_rate = self.track.file_codec_parameters().sample_rate.unwrap() as f64;
        let total_samples_spanned = time_span * samp_rate;
        let step = (total_samples_spanned / 500.0) as usize + 1;
        let time_per_step = step as f64 / samp_rate;
        let time_per_sample = 1.0 / samp_rate;

        // Sample over an appropriate data range to get coords for
        let rough_start = ((range.start() * samp_rate) as usize).saturating_sub(1);
        let rough_end =
            ((range.end() * samp_rate) as usize + 1).min(self.track.file_data().0.len());

        let coords_left: Vec<_> = self.track.file_data().0[rough_start..rough_end]
            .chunks(step)
            .enumerate()
            .filter_map(|(x, chunk)| {
                let x64 = (x as f64 * time_per_step) + (rough_start as f64 * time_per_sample);
                range
                    .contains(&x64)
                    .then(|| [x64, get_extreme(chunk) as f64 / 2.0 + 0.5])
            })
            .collect();

        let coords_right: Vec<_> = self.track.file_data().1[rough_start..rough_end]
            .chunks(step)
            .enumerate()
            .filter_map(|(x, chunk)| {
                let x64 = (x as f64 * time_per_step) + (rough_start as f64 * time_per_sample);
                range
                    .contains(&x64)
                    .then(|| [x64, get_extreme(chunk) as f64 / 2.0 - 0.5]) // + 1.0 for being the second track
            })
            .collect();

        // Plot things
        let line_left = egui_plot::Line::new("left", coords_left)
            .fill(0.5)
            .color(egui::Color32::PURPLE)
            .fill_alpha(0.4);

        let line_right = egui_plot::Line::new("right", coords_right)
            .fill(-0.5)
            .color(egui::Color32::BLUE)
            .fill_alpha(0.4);

        egui_plot::Plot::new("waveform")
            .legend(egui_plot::Legend::default())
            .clamp_grid(false)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .center_y_axis(true)
            .id(plot_id)
            .height(300.0)
            .default_y_bounds(-1.0, 1.0)
            .show(ui, |plot_ui| {
                plot_ui.line(line_left);
                plot_ui.line(line_right);
            })
            .response
    }
}
