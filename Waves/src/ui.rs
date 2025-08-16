use eframe::egui::{self, Response, Widget};
use egui_plot::GridMark;
use symphonia::core::errors::Error;

use std::{
    mem,
    ops::RangeInclusive,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    thread::JoinHandle,
};

use crate::{
    audio::AudioCommand,
    common::{self, Channel, track::Track},
};

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
    track: &'a Arc<Track>,
    current_sample: usize,
    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
    tx_commands: Sender<AudioCommand>,
}

impl<'a> WaveformWidget<'a> {
    pub fn new(
        track: &'a Arc<Track>,
        current_sample: usize,
        tx_commands: Sender<AudioCommand>,
    ) -> Self {
        Self {
            track,
            current_sample,
            _vertical: true,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
            tx_commands,
        }
    }

    fn compute_line_data(
        &self,
        range: RangeInclusive<f64>,
        samp_rate: f64,
        channel: Channel,
    ) -> Vec<egui_plot::Line<'_>> {
        const FILLED_LIMIT: usize = 32;

        let rough_start =
            ((range.start() * samp_rate) as usize).min(self.track.sample_data().0.len() - 1);
        let rough_end =
            ((range.end() * samp_rate) as usize + 1).min(self.track.sample_data().0.len());

        let time_per_sample = 1.0 / samp_rate;

        let (data, l_step, f_step) = match channel {
            Channel::Left => self
                .track
                .file_data_left()
                .get_presampled_data_and_step(rough_end - rough_start),
            Channel::Right => self
                .track
                .file_data_right()
                .get_presampled_data_and_step(rough_end - rough_start),
        };

        let offset_func = match channel {
            Channel::Left => |f: f64| f / 2.0 + 0.5,
            Channel::Right => |f: f64| f / 2.0 - 0.5,
        };

        let fill_func = match channel {
            Channel::Left => || 0.5,
            Channel::Right => || -0.5,
        };

        let max_alpha = 0.8;
        let min_alpha = 0.3;

        let line_data: Vec<egui_plot::Line<'_>> = match data.len() {
            1 => {
                // This hapens if we expect to just draw the line
                let coords_left: Vec<_> = data[0][rough_start / l_step..rough_end / l_step]
                    .iter()
                    .enumerate()
                    .filter_map(|(x, y)| {
                        let x64 = ((x * l_step) as f64 * time_per_sample)
                            + (rough_start as f64 * time_per_sample);
                        range.contains(&x64).then(|| [x64, offset_func(*y as f64)])
                        // + 1.0 for being the second track
                    })
                    .collect();

                let frac = 2.0f32.powf(f_step) / FILLED_LIMIT as f32;

                // Plot things
                vec![
                    egui_plot::Line::new("left", coords_left)
                        .fill(fill_func())
                        .color(egui::Color32::PURPLE)
                        .fill_alpha(min_alpha * (1.0 - frac) + max_alpha * (frac)),
                ]
            }
            2 => {
                // We run this if we want to draw both the max and the min funcs

                let raw_coords_max = data[1][rough_start / l_step..rough_end / l_step]
                    .iter()
                    .enumerate()
                    .filter_map(|(x, y)| {
                        let x64 = ((x * l_step) as f64 * time_per_sample)
                            + (rough_start as f64 * time_per_sample);
                        range.contains(&x64).then(|| [x64, *y as f64]) // + 1.0 for being the second track
                    });

                let raw_coords_min = data[0][rough_start / l_step..rough_end / l_step]
                    .iter()
                    .enumerate()
                    .filter_map(|(x, y)| {
                        let x64 = ((x * l_step) as f64 * time_per_sample)
                            + (rough_start as f64 * time_per_sample);
                        range.contains(&x64).then(|| [x64, *y as f64]) // + 1.0 for being the second track
                    });

                let coords_max: Vec<_> = raw_coords_max.map(|[x, y]| [x, offset_func(y)]).collect();

                let coords_min: Vec<_> = raw_coords_min.map(|[x, y]| [x, offset_func(y)]).collect();

                //println!("{:?},\n\n\n{:?}", coords_max, coords_min);

                // Get top and bottom lines
                let up = egui_plot::Line::new("left", coords_max)
                    .fill(fill_func())
                    .color(egui::Color32::PURPLE)
                    .fill_alpha(max_alpha);

                let down = egui_plot::Line::new("left", coords_min)
                    .fill(fill_func())
                    .color(egui::Color32::PURPLE)
                    .fill_alpha(max_alpha);

                vec![up, down]
            }
            _ => {
                eprintln!("we have more than two or zero of line/min/max samples");
                vec![]
            }
        };

        line_data
    }
}

impl Widget for WaveformWidget<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let plot_id = ui.id();

        let samp_rate = self.track.sample_rate() as f64;
        let time_per_sample = 1.0 / samp_rate;
        let track_len = self.track.length() as f64;

        // Initialise data eg getting start stop times and step size
        let range = if let Some(plot_memory) = egui_plot::PlotMemory::load(ui.ctx(), plot_id) {
            let r = plot_memory.bounds().range_x();
            let three_samples = 3.0 / samp_rate;
            (r.start() - three_samples).clamp(0.0, track_len)
                ..=(r.end() + three_samples).clamp(0.0, track_len)
        } else {
            0.02f64..=1000.0f64
        };

        // Do Left

        let line_left = self.compute_line_data(range.clone(), samp_rate, Channel::Left);
        let line_right = self.compute_line_data(range.clone(), samp_rate, Channel::Right);

        // Draw the timestamp line
        let line_time = egui_plot::Line::new(
            "time",
            vec![
                [self.current_sample as f64 * time_per_sample, 1.0],
                [self.current_sample as f64 * time_per_sample, -1.0],
            ],
        )
        .color(egui::Color32::WHITE);

        let plt = egui_plot::Plot::new("waveform")
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
                for l in line_left {
                    plot_ui.line(l);
                }
                for l in line_right {
                    plot_ui.line(l);
                }
                plot_ui.line(line_time);
                plot_ui.pointer_coordinate()
            });

        if plt.response.clicked() {
            if let Some(coord) = plt.inner {
                let x_time = coord.x.max(0.0).min(track_len * time_per_sample);

                let x_sample = (x_time * samp_rate) as usize;
                self.tx_commands
                    .send(AudioCommand::RelocateTo(self.track.clone(), x_sample))
                    .expect("Can't reset time");

                self.current_sample = x_sample;
            }
        }

        plt.response
    }
}

pub struct EQWidget<'a> {
    track: &'a Arc<Track>,
    data_width: usize,
    current_sample: usize,
    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
}

impl<'a> EQWidget<'a> {
    pub fn new(track: &'a Arc<Track>, sample_count: usize, current_sample: usize) -> Self {
        Self {
            track,
            data_width: sample_count,
            current_sample,
            _vertical: true,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
        }
    }
}

impl Widget for EQWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // Get the frequency data from the sample data, and have it centred on the current sample
        // If the data does not fully cover then assume it is zero.

        let sample_rate = self.track.sample_rate();
        let current_range = (self.current_sample as i32 - self.data_width as i32 / 2)
            ..(self.current_sample as i32 + self.data_width as i32 / 2);

        let track_len = self.track.length() as i32;
        let mut useful_sample_buffer;

        let useful_samples = if current_range.start < 0 || current_range.end >= track_len {
            useful_sample_buffer = vec![0.0; self.data_width];

            let start_in_track = current_range.start.max(0) as usize;
            let mut start_in_useful = 0;
            if current_range.start < 0 {
                start_in_useful = -current_range.start as usize;
            }

            let end_in_track = current_range.end.min(track_len) as usize;
            let mut end_in_useful = self.data_width;
            if current_range.end >= track_len {
                end_in_useful = self.data_width - (current_range.end - track_len) as usize;
            }

            useful_sample_buffer[start_in_useful..end_in_useful]
                .copy_from_slice(&self.track.sample_data().0[start_in_track..end_in_track]);

            &useful_sample_buffer[..]
        } else {
            &self.track.sample_data().0[current_range.start as usize..current_range.end as usize]
        };

        let freq_data = common::fft(useful_samples);
        //println!("{:?}", freq_data);

        // println!(
        //     "{}, {}",
        //     useful_samples.into_iter().sum::<f32>(),
        //     (useful_samples.into_iter().sum::<f32>() / self.data_width as f32).log2() + 6.0
        // );

        let coords: Vec<_> = freq_data[0..(self.data_width as usize / 2)]
            .iter()
            .enumerate()
            .map(|(i, &f)| [(i as f64).log2(), ((f as f64) / 5.0).log2() + 6.0])
            .collect();

        let freq_line = egui_plot::Line::new("frequency", coords)
            .fill(-18.0)
            .color(egui::Color32::GREEN)
            .fill_alpha(0.4);

        let max_x = (self.data_width as f64).log2() - 1.0;
        let min_x = max_x * 0.29;

        // Recall that the maximum frequency shown is sample rate / 2 so we can draw grid lines now

        let plt = egui_plot::Plot::new("fourier_transform")
            .legend(egui_plot::Legend::default())
            .clamp_grid(false)
            .show_grid(true)
            .x_grid_spacer(|_input| {
                //
                let x_coords: Vec<GridMark> = [
                    20.0, 30.0, 100.0, 200.0, 300.0, 1000.0, 2000.0, 3000.0, 10000.0, 20000.0,
                ]
                .map(|f: f64| (f.log2() * max_x) / ((sample_rate as f64).log2() - 1.0))
                .map(|v| GridMark {
                    value: v,
                    step_size: 100.0,
                })
                .to_vec();

                x_coords
            })
            .y_grid_spacer(|_input| {
                let y_coords: Vec<GridMark> = [12.0, 3.0, 0.0, -3.0, -12.0]
                    .map(|v| GridMark {
                        value: v,
                        step_size: 100.0,
                    })
                    .to_vec();

                y_coords
            })
            .x_axis_formatter(|a, _range| {
                let str = format!("{:.0}", (sample_rate as f64 / 2.0).powf(a.value / max_x));
                match str.as_str() {
                    "10000" => "10k".to_string(),
                    "20000" => "20k".to_string(),
                    _ => str,
                }
            })
            .center_y_axis(true)
            .height(300.0)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .label_formatter(|_name, value| {
                let freq_pos = (sample_rate as f64 / 2.0).powf(value.x as f64 / max_x);
                format!("frequency: {:.2} \nvolume: {:.2}", freq_pos, value.y)
            })
            .center_y_axis(true)
            .default_x_bounds(min_x, max_x)
            .default_y_bounds(-18.0, 18.0)
            .show(ui, |plot_ui| {
                plot_ui.line(freq_line);
                plot_ui.pointer_coordinate();
            });

        plt.response
    }
}
