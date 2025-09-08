use crate::{
    audio::effects::{Effect, output::Output},
    common::{Channel, mipmapchannel::SamplePlotData, track::Track},
    player::AudioCommand,
};
use eframe::egui::{self, Widget};
use std::{
    ops::RangeInclusive,
    sync::{Arc, mpsc::Sender},
};

/// Want to be able to build a waveform widget that displays the waveform after applying the effect
/// We will want some apply_sampled_data function like apply in each effect to be able to run (and also use the mipmap functionaility)
pub struct WaveformWidget {
    current_sample: usize,
    effect: Arc<dyn Effect>,
    plot_size: (f32, f32),
    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
    is_small_widget: bool,
    tx_commands: Option<Sender<AudioCommand>>,
}

impl WaveformWidget {
    pub fn new(
        current_sample: usize,
        effect: Arc<dyn Effect>,
        plot_size: (f32, f32),
        interactable: bool,
        tx_commands: Option<Sender<AudioCommand>>,
    ) -> Self {
        match interactable {
            false => Self {
                current_sample,
                effect,
                _vertical: true,
                plot_size,
                allow_zoom: false.into(),
                allow_drag: false.into(),
                allow_scroll: false.into(),
                is_small_widget: true,
                tx_commands,
            },
            true => Self {
                current_sample,
                effect,
                _vertical: true,
                plot_size,
                allow_zoom: [true, false].into(),
                allow_drag: [true, false].into(),
                allow_scroll: [true, false].into(),
                is_small_widget: false,
                tx_commands,
            },
        }
    }

    fn get_start_sample(&self, data_width: usize, step: usize) -> usize {
        // Max 100 minutes
        let sample_count: usize = 48000 * 72 * 60;
        // Get where start should be considering lower
        let lower = self.current_sample.saturating_sub((data_width / 2) * step);

        // Get where start should be considering higher
        let upper = self
            .current_sample
            .min(sample_count.saturating_sub(data_width * step));

        lower.min(upper)
    }

    pub fn compute_line_data_from_effect(
        &self,
        samp_rate: f64,
        data_width: usize,
        step: usize,
        channel: Channel,
    ) -> Vec<egui_plot::Line<'_>> {
        const FILLED_LIMIT: usize = 32;

        let start_sample = self.get_start_sample(data_width, step);

        let mut sample_plot_data = SamplePlotData::new(step, start_sample, data_width);

        // do the maths to get the plot data back
        self.effect
            .get_waveform_plot_data(&mut sample_plot_data, &channel);

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

        let data = sample_plot_data.data;
        let time_per_sample = 1.0 / samp_rate;

        let range = 0.0..=(data_width as f64 * step as f64 * time_per_sample);

        let line_data: Vec<egui_plot::Line<'_>> = match data.len() {
            1 => {
                // This hapens if we expect to just draw the line
                let coords_left: Vec<_> = data[0]
                    .iter()
                    .enumerate()
                    .filter_map(|(x, y)| {
                        let x64 = ((x * step) as f64 * time_per_sample) + (range.start());
                        range.contains(&x64).then(|| [x64, offset_func(*y as f64)])
                        // + 1.0 for being the second track
                    })
                    .collect();

                let frac = 2.0f32.powf(step as f32) / FILLED_LIMIT as f32;

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

                let raw_coords_max = data[1].iter().enumerate().filter_map(|(x, y)| {
                    let x64 = ((x * step) as f64 * time_per_sample) + (range.start());
                    range.contains(&x64).then(|| [x64, *y as f64]) // + 1.0 for being the second track
                });

                let raw_coords_min = data[0].iter().enumerate().filter_map(|(x, y)| {
                    let x64 = ((x * step) as f64 * time_per_sample) + (range.start());
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

    fn get_small_line_data(
        &self,
        samp_rate: f64,
        data_width: usize,
        step: usize,
    ) -> (
        Vec<egui_plot::Line<'_>>,
        Vec<egui_plot::Line<'_>>,
        Option<egui_plot::Line<'_>>,
    ) {
        let time_per_sample = 1.0 / samp_rate;
        // deal with say 512 datapoints want to get some step size and the data back

        let line_left =
            self.compute_line_data_from_effect(samp_rate, data_width, step, Channel::Left);
        let line_right =
            self.compute_line_data_from_effect(samp_rate, data_width, step, Channel::Right);

        let start_sample = self.get_start_sample(data_width, step);
        let max_difference = step * data_width;

        // Draw the timestamp line if its relevant
        let mut line_time = None;
        if (0..=max_difference).contains(&(self.current_sample - start_sample)) {
            line_time = Some(
                egui_plot::Line::new(
                    "time",
                    vec![
                        [
                            (self.current_sample - start_sample) as f64 * time_per_sample,
                            1.0,
                        ],
                        [
                            (self.current_sample - start_sample) as f64 * time_per_sample,
                            -1.0,
                        ],
                    ],
                )
                .color(egui::Color32::WHITE),
            );
        };

        (line_left, line_right, line_time)
    }

    fn get_big_line_data(
        &self,
        range: RangeInclusive<f64>,
        samp_rate: f64,
        data_width: usize,
    ) -> (
        Vec<egui_plot::Line<'_>>,
        Vec<egui_plot::Line<'_>>,
        Option<egui_plot::Line<'_>>,
    ) {
        let time_per_sample = 1.0 / samp_rate;
        // deal with say 512 datapoints want to get some step size and the data back
        let exact_step = samp_rate * (range.end() - range.start()) / data_width as f64;
        let log = exact_step.log2().floor();
        let step = ((2.0f64).powf(log) as usize);

        // Do Left

        let line_left =
            self.compute_line_data_from_effect(samp_rate, data_width, step, Channel::Left);
        let line_right =
            self.compute_line_data_from_effect(samp_rate, data_width, step, Channel::Right);

        // Draw the timestamp line
        let line_time = egui_plot::Line::new(
            "time",
            vec![
                [self.current_sample as f64 * time_per_sample, 1.0],
                [self.current_sample as f64 * time_per_sample, -1.0],
            ],
        )
        .color(egui::Color32::WHITE);

        (line_left, line_right, Some(line_time))
    }
}

impl WaveformWidget {
    pub fn ui(mut self, ui: &mut egui::Ui, show_current_sample: bool) -> egui::Response {
        let plot_id = ui.id();

        let samp_rate = 48000.0;
        let time_span = 20.0 * 60.0;

        let (line_left, line_right, line_time) = match self.is_small_widget {
            true => self.get_small_line_data(samp_rate, 256, 2048),
            false => {
                // Initialise data eg getting start stop times and step size
                let range =
                    if let Some(plot_memory) = egui_plot::PlotMemory::load(ui.ctx(), plot_id) {
                        let r = plot_memory.bounds().range_x();
                        let three_samples = 3.0 / samp_rate;
                        (r.start() - three_samples).clamp(0.0, time_span * samp_rate)
                            ..=(r.end() + three_samples).clamp(0.0, time_span * samp_rate)
                    } else {
                        0.02f64..=1000.0f64
                    };
                self.get_big_line_data(range, samp_rate, 1024)
            }
        };

        let plt = egui_plot::Plot::new("waveform")
            .legend(egui_plot::Legend::default())
            .clamp_grid(false)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .center_y_axis(true)
            .id(plot_id)
            .width(self.plot_size.0)
            .height(self.plot_size.1)
            .show_x(false)
            .show_y(false)
            .show_axes(false)
            .show_grid(false)
            .default_y_bounds(-1.0, 1.0)
            .show(ui, |plot_ui| {
                for l in line_left {
                    plot_ui.line(l);
                }
                for l in line_right {
                    plot_ui.line(l);
                }
                if show_current_sample {
                    match line_time {
                        None => (),
                        Some(line_time) => plot_ui.line(line_time),
                    };
                }

                plot_ui.pointer_coordinate()
            });

        if plt.response.clicked() {
            if let Some(coord) = plt.inner {
                let x_time = coord.x.max(0.0); //.min(track_len * time_per_sample);

                let x_sample = (x_time * samp_rate) as usize;
                match self.tx_commands {
                    None => (),
                    Some(tx_commands) => {
                        tx_commands
                            .send(AudioCommand::RelocateTo(self.effect.clone(), x_sample))
                            .expect("Can't reset time");
                    }
                }

                self.current_sample = x_sample;
            }
        }

        plt.response
    }
}
