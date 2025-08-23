use std::sync::Arc;

use eframe::egui::{self, Widget};
use egui_plot::{GridMark, Line};

use crate::common::{self, dB, track::Track};

pub struct EQWidget {
    sample_data: Vec<f32>,
    data_width: usize,
    sample_rate: u32,

    plot_height: f32,
    plot_width: f32,

    _vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
}

impl EQWidget {
    pub fn new(sample_data: Vec<f32>, sample_rate: u32, plot_size: (f32, f32)) -> Self {
        Self {
            data_width: sample_data.len(),
            sample_data,
            sample_rate,
            plot_height: plot_size.1,
            plot_width: plot_size.0,
            _vertical: true,
            allow_zoom: [true, false].into(),
            allow_drag: [true, false].into(),
            allow_scroll: [true, false].into(),
        }
    }

    pub fn _new_from_track(track: &Arc<Track>, sample_count: usize, current_sample: usize) -> Self {
        let data_width = sample_count;
        let sample_rate = track.sample_rate();
        let current_range = (current_sample as i32 - data_width as i32 / 2)
            ..(current_sample as i32 + data_width as i32 / 2);

        let track_len = track.length() as i32;
        let mut useful_sample_buffer;

        let useful_samples = if current_range.start < 0 || current_range.end >= track_len {
            useful_sample_buffer = vec![0.0; data_width];

            let start_in_track = current_range.start.max(0) as usize;
            let mut start_in_useful = 0;
            if current_range.start < 0 {
                start_in_useful = -current_range.start as usize;
            }

            let end_in_track = current_range.end.min(track_len) as usize;
            let mut end_in_useful = data_width;
            if current_range.end >= track_len {
                end_in_useful = data_width - (current_range.end - track_len) as usize;
            }

            useful_sample_buffer[start_in_useful..end_in_useful]
                .copy_from_slice(&track.sample_data().0[start_in_track..end_in_track]);

            &useful_sample_buffer[..]
        } else {
            &track.sample_data().0[current_range.start as usize..current_range.end as usize]
        };

        Self::new(useful_samples.to_vec(), sample_rate, (150.0, 75.0))
    }

    fn get_freq_line(&self) -> Line<'_> {
        let freq_data;
        {
            let scope = tracing::trace_span!("fft_draw");
            let _span = scope.enter();
            freq_data = common::fft(&self.sample_data);
        }

        //println!("{:?} {}", freq_data, freq_data.len());

        let coords: Vec<_> = freq_data[0..(self.data_width as usize / 2)]
            .iter()
            .enumerate()
            .map(|(i, &f)| {
                [
                    {
                        match i {
                            0 => (0.01f64).log2(),
                            _ => (i as f64).log2(),
                        }
                    },
                    dB::from_amplitude(f).0 as f64,
                ]
            })
            .collect();

        let freq_line = egui_plot::Line::new("frequency", coords)
            .fill(-18.0)
            .color(egui::Color32::GREEN)
            .fill_alpha(0.4);

        freq_line
    }
}

impl Widget for EQWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let scope = tracing::trace_span!("drawing_freq_plot");
        let _span = scope.enter();
        // Get the frequency data from the sample data, and have it centred on the current sample
        // If the data does not fully cover then assume it is zero.

        //println!("{:?}", freq_data);

        // println!(
        //     "{}, {}",
        //     useful_samples.into_iter().sum::<f32>(),
        //     (useful_samples.into_iter().sum::<f32>() / self.data_width as f32).log2() + 6.0
        // );

        let freq_line = self.get_freq_line();

        let max_x =
            22000.0f64.log2() - (self.sample_rate as f64).log2() + (self.data_width as f64).log2();
        let min_x =
            18.0f64.log2() - (self.sample_rate as f64).log2() + (self.data_width as f64).log2();

        // Recall that the maximum frequency shown is sample rate / 2 so we can draw grid lines now

        let plt = egui_plot::Plot::new("fourier_transform")
            //.legend(egui_plot::Legend::default())
            .clamp_grid(false)
            .show_grid(true)
            .x_grid_spacer(|_input| {
                //
                let x_coords: Vec<GridMark> = [
                    20.0, 30.0, 100.0, 200.0, 300.0, 1000.0, 2000.0, 3000.0, 10000.0, 20000.0,
                ]
                .map(|f: f64| {
                    f.log2() - (self.sample_rate as f64).log2()
                        + (self.sample_data.len() as f64).log2()
                })
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
            // .x_axis_formatter(|a, _range| {
            //     let str = format!(
            //         "{:.0}",
            //         (self.sample_rate as f64 / 2.0).powf(a.value / max_x)
            //     );
            //     match str.as_str() {
            //         "10000" => "10k".to_string(),
            //         "20000" => "20k".to_string(),
            //         _ => str,
            //     }
            // })
            .show_x(false)
            .show_y(false)
            .show_axes(false)
            .center_y_axis(true)
            .height(self.plot_height)
            .width(self.plot_width)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            // .label_formatter(|_name, value| {
            //     let freq_pos = (self.sample_rate as f64 / 2.0).powf(value.x as f64 / max_x);
            //     format!("frequency: {:.2} \nvolume: {:.2}", freq_pos, value.y)
            // })
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
