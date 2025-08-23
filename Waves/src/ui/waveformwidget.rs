use crate::{
    audio::effects::Effect,
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
    plot_size: (f32, f32),
    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
    tx_commands: Option<Sender<AudioCommand>>,
}

impl WaveformWidget {
    pub fn new(
        current_sample: usize,
        plot_size: (f32, f32),
        tx_commands: Option<Sender<AudioCommand>>,
    ) -> Self {
        Self {
            current_sample,
            _vertical: true,
            plot_size,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
            tx_commands,
        }
    }

    pub fn compute_line_data_from_effect(
        &self,
        effect: &Arc<dyn Effect>,
        samp_rate: f64,
        channel: Channel,
    ) -> Vec<egui_plot::Line<'_>> {
        const FILLED_LIMIT: usize = 32;

        // deal with say 512 datapoints want to get some step size and the data back
        let step = 2048;
        let data_width = 256;

        let start_sample = self.current_sample.saturating_sub(data_width / 2);

        let mut sample_plot_data = SamplePlotData::new(step, start_sample, data_width);

        // do the maths to get the plot data back
        effect.get_waveform_plot_data(&mut sample_plot_data, &channel);

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

        //println!("{:?}", data);

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
}

impl WaveformWidget {
    pub fn ui(
        mut self,
        ui: &mut egui::Ui,
        effect: Arc<dyn Effect>,
        show_current_sample: bool,
    ) -> egui::Response {
        let plot_id = ui.id();

        let samp_rate = 48000.0;
        let time_per_sample = 1.0 / samp_rate;

        // Do Left

        let line_left = self.compute_line_data_from_effect(&effect, samp_rate, Channel::Left);
        let line_right = self.compute_line_data_from_effect(&effect, samp_rate, Channel::Right);

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
                    plot_ui.line(line_time);
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
                        // tx_commands
                        //     .send(AudioCommand::RelocateTo(self.effect.clone(), x_sample))
                        //     .expect("Can't reset time");
                    }
                }

                self.current_sample = x_sample;
            }
        }

        plt.response
    }
}
