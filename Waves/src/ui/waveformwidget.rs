use crate::{
    common::{Channel, track::Track},
    player::AudioCommand,
};
use eframe::egui::{self, Widget};
use std::{
    ops::RangeInclusive,
    sync::{Arc, mpsc::Sender},
};

pub struct WaveformWidget<'a> {
    track: &'a Arc<Track>,
    current_sample: usize,
    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
    tx_commands: Option<Sender<AudioCommand>>,
}

impl<'a> WaveformWidget<'a> {
    pub fn new(
        track: &'a Arc<Track>,
        current_sample: usize,
        tx_commands: Option<Sender<AudioCommand>>,
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
                match self.tx_commands {
                    None => (),
                    Some(tx_commands) => {
                        // tx_commands
                        //     .send(AudioCommand::RelocateTo(self.track.clone(), x_sample))
                        //     .expect("Can't reset time");
                    }
                }

                self.current_sample = x_sample;
            }
        }

        plt.response
    }
}
