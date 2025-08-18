use std::sync::Arc;

use eframe::egui::{self, Widget};
use egui_plot::GridMark;

use crate::common::{self, track::Track};

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
