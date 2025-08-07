use eframe::egui;

use std::{
    sync::{Arc, mpsc},
    thread,
};
use symphonia::core::audio::Signal;

mod audio;
mod common;
mod loader;

use common::Track;

use crate::audio::AudioThread;

struct ProgressTracker {
    prog: f32,

    tx: mpsc::Sender<f32>,
    rx: mpsc::Receiver<f32>,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { prog: 0.0, tx, rx }
    }
}

impl ProgressTracker {
    fn update(&mut self) {
        while let Ok(p) = self.rx.try_recv() {
            self.prog = p;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.prog > 0.0 {
            ui.add(egui::ProgressBar::new(self.prog));
        }
    }
}

#[derive(PartialEq)]
struct WaveformWidget {
    vertical: bool,
    allow_zoom: egui::Vec2b,
    allow_drag: egui::Vec2b,
    allow_scroll: egui::Vec2b,
}

impl Default for WaveformWidget {
    fn default() -> Self {
        Self {
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

impl WaveformWidget {
    fn draw_widget(&self, ui: &mut egui::Ui, waveform: &Track) -> egui::Response {
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
            .default_y_bounds(-1.0, 1.0)
            .show(ui, |plot_ui| plot_ui.line(line))
            .response
    }
}

struct MyEguiApp {
    active: Option<Arc<Track>>,
    prog: ProgressTracker,
    tx_loader: mpsc::Sender<Track>,
    rx_loader: mpsc::Receiver<Track>,
    audio_thread: audio::AudioThread,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let (tx, rx) = mpsc::channel();

        Self {
            tx_loader: tx,
            rx_loader: rx,
            active: Default::default(),
            prog: Default::default(),
            audio_thread: AudioThread::new(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // UI
            ui.heading("Hello World!");

            let cd: WaveformWidget = WaveformWidget::default();

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx_loader.try_recv() {
                self.active = Some(Arc::new(rx));
            }

            if let Some(t) = self.active.as_ref() {
                ui.label(format!("{:?}", t.file_path()));
                ui.label(format!("{:?}", t.file_codec_parameters()));

                cd.draw_widget(ui, &t);

                if ui.button("play").clicked() {
                    // Output

                    self.audio_thread.send_command(audio::AudioCommand::Stop);
                    // arc and clone because threading (pretty much)
                    self.audio_thread
                        .send_command(audio::AudioCommand::PlayFromSample(t.clone(), 0));
                }
            } else {
                self.prog.update();
                self.prog.ui(ui);
            }

            //ui.label(format!("{:?}", self.active_file_samples));

            /////////////////////////////

            // Rip the data from the file (drag and drop)
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            // turn vector into a slice and reference it (because referencing is required for slices)
            match &dropped[..] {
                [file] => {
                    // wizardry.
                    // We get file path and share the ownership of ARC (the memory holding the buffer data)
                    // Then we can use a thread to not be held up at this point till the file is loaded
                    // After thats done we then set the data in the thread because it aquires a lock (ie noone else is editing)
                    // Close the thread
                    // This is only ok because we have given the thread (through the arc and mutex) control.
                    let file_path = file.path.clone().expect("Web not supported.");
                    let tx = self.tx_loader.clone();
                    let prog = self.prog.tx.clone();
                    thread::spawn(move || {
                        let track = Track::get_data_from_mp3_path(file_path.clone(), prog);

                        // let mut file_left_data = Vec::new();
                        // let mut file_right_data = Vec::new();

                        // for (i, packet) in file_buffer.iter().enumerate() {
                        //     file_left_data.extend_from_slice(packet.chan(0));
                        //     file_right_data.extend_from_slice(packet.chan(1));

                        //     let packet_size = packet.capacity();
                        //     println!("{packet_count}, {packet_size}");

                        //     let progress = TrackLoad {
                        //         track: None,
                        //         progress: (i as f32 / packet_count),
                        //     };
                        //     tx.send(progress).unwrap();
                        // }

                        tx.send(track).unwrap();
                    });
                }
                [_file, ..] => println!("Multiple Files inputted!"),
                _ => (),
            }
        });
    }
}

fn main() {
    // Activate drag and drop (not necessary)
    let native_options = eframe::NativeOptions::default();
    native_options.viewport.with_drag_and_drop(true);

    // if its native
    let _ = eframe::run_native(
        "My egui App",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );

    // maybe make it web at some point
}
