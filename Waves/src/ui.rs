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
            ui.add(egui::ProgressBar::new(self.progress).desired_width(50.0))
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

impl WaveformWidget<'_> {
    pub fn draw_widget(&self, ui: &mut egui::Ui, waveform: &Track) -> egui::Response {
        let id = ui.id();

        let range = if let Some(plot_memory) = egui_plot::PlotMemory::load(ui.ctx(), id) {
            plot_memory.bounds().range_x()
        } else {
            0.02..=1000.0
        };

        let time_span = range.end() - range.start();
        let samp_rate = waveform.file_codec_parameters().sample_rate.unwrap() as f64;
        let total_samples_spanned = time_span * samp_rate;
        let step = (total_samples_spanned / 500.0) as usize + 1;
        let time_per_step = step as f64 / samp_rate;
        let time_per_sample = 1.0 / samp_rate;

        // Goal is to force the stepsize to be close to multiples / factors of the packet_size
        // to make things efficient and reduce the possibility of artefacts
        // we will also try a way to make things not bunch up on any one side

        // let relative_step_to_packet = true_step as f64 / packet_size as f64;
        // let log_value = relative_step_to_packet.log2();
        // let step = (2 >> log_value.round() as i32) as usize * packet_size;

        // println!("{true_step}, {packet_size}, {step}");

        // Now lets chunk the packets up to the size of true_step and get some useful data
        // let plot_data;

        // if (log_value < 1.0)
        // {
        //     plot_data = waveform.file_buffer.iter().map(|packet| {packet.chan(0).chunks(step).map(get_extreme)})
        // } else {
        //     let basic_plot_data = waveform.file_buffer.iter().map(|packet| {packet.chan(0).chunks(step).map(get_extreme)})
        // }

        // let mut chart = egui_plot::BarChart::new(
        //     format!("{:?}", waveform.file_path),
        //     waveform
        //         .file_left_data
        //         .chunks(step)
        //         .map(|chunk| get_extreme(chunk))
        //         .enumerate()
        //         .filter_map(|(x, y)| {
        //             let x64 = x as f64 * time_per_step;
        //             range.contains(&x64).then(|| (x64, y as f64))
        //         })
        //         .map(|(x, y)| egui_plot::Bar::new(x, y).width(time_per_step))
        //         .collect(),
        // )
        // .color(Color32::LIGHT_BLUE);

        // Get the wave points data we want

        let rough_start = ((range.start() * samp_rate) as usize).saturating_sub(1);
        let rough_end = ((range.end() * samp_rate) as usize + 1).min(waveform.file_data().0.len());

        let coords: Vec<_> = waveform.file_data().0[rough_start..rough_end]
            .chunks(step)
            .enumerate()
            .filter_map(|(x, chunk)| {
                let x64 = (x as f64 * time_per_step) + (rough_start as f64 * time_per_sample);
                range
                    .contains(&x64)
                    .then(|| [x64, get_extreme(chunk) as f64])
            })
            .collect();

        let line = egui_plot::Line::new("Waveform", coords)
            .fill(0.0)
            .color(egui::Color32::PURPLE)
            .fill_alpha(0.4);

        egui_plot::Plot::new("Normal Distribution Demo")
            .legend(egui_plot::Legend::default())
            .clamp_grid(false)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .id(id)
            .center_y_axis(true)
            .height(300.0)
            .default_y_bounds(-1.0, 1.0)
            .show(ui, |plot_ui| plot_ui.line(line))
            .response
    }
}
