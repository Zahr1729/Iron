use eframe::egui;

use std::{
    sync::{Arc, mpsc},
    thread,
};

mod audio;
mod common;
mod loader;
mod ui;

use common::Track;

use crate::audio::AudioThread;

struct MyEguiApp {
    active: Option<Arc<Track>>,
    prog: ui::ProgressTracker,
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

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx_loader.try_recv() {
                self.active = Some(Arc::new(rx));
            }

            if let Some(t) = self.active.as_ref() {
                ui.label(format!("{:?}", t.file_path()));
                ui.label(format!("{:?}", t.file_codec_parameters()));

                let cd = ui::WaveformWidget::new(t);
                cd.draw_widget(ui, &t);

                if ui.button("play").clicked() {
                    // Output
                    self.audio_thread.send_command(audio::AudioCommand::Stop);
                    // arc and clone because threading (pretty much)
                    self.audio_thread
                        .send_command(audio::AudioCommand::PlayFromSample(t.clone(), 0));
                }

                if ui.button("stop").clicked() {
                    // Output
                    self.audio_thread.send_command(audio::AudioCommand::Stop);
                }
            } else {
                ui.add(&mut self.prog);
            }

            //ui.label(format!("{:?}", self.active_file_samples));

            /////////////////////////////

            // Rip the data from the file (drag and drop)
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            // turn vector into a slice and reference it (because referencing is required for slices)
            match &dropped[..] {
                [file] => {
                    let file_path = file.path.clone().expect("Web not supported.");
                    let tx = self.tx_loader.clone();
                    let prog = self.prog.tx.clone();
                    thread::spawn(move || {
                        let track = Track::get_data_from_mp3_path(file_path.clone(), prog);

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
