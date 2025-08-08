use eframe::egui;
use symphonia::core::errors::Error;

use std::{
    sync::{Arc, mpsc},
    thread,
};

mod audio;
mod common;
mod loader;
mod ui;

use common::Track;

use crate::{
    audio::AudioThread,
    ui::{ProgressTracker, ThreadTracker},
};

struct MyEguiApp {
    active_track: Option<Arc<Track>>,
    tx_loader: mpsc::Sender<Track>,
    rx_loader: mpsc::Receiver<Track>,
    audio_thread: audio::AudioThread,
    // Must store
    // - widget
    //   - progress bar if in progressed (not completed)
    //   - error message (if panicked)
    // - thread handle, so we can clear out finished ops
    ops_in_progress: Vec<ThreadTracker>,
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
            active_track: Default::default(),
            audio_thread: AudioThread::new(),
            ops_in_progress: Default::default(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Bottom Panel
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            // Iterate through all progress bars and display on the bottom of the screen

            for bar in &mut self.ops_in_progress {
                bar.check_is_done();
            }

            self.ops_in_progress
                .retain_mut(|thread_tracker| !thread_tracker.should_dismiss);

            ui.horizontal(|ui| {
                for bar in &mut self.ops_in_progress {
                    ui.add(bar);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // UI
            ui.heading("Hello World!");

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx_loader.try_recv() {
                self.active_track = Some(Arc::new(rx));
            }

            if let Some(t) = self.active_track.as_ref() {
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

                    let new_op_progress_bar = ProgressTracker::default();
                    let thread_name = file_path.file_name().unwrap().to_string_lossy().to_string();

                    let prog_sender = new_op_progress_bar.tx.clone();

                    // thread to do the loading file yippee!
                    let handle = thread::spawn(move || -> Result<(), Error> {
                        let track = Track::get_data_from_mp3_path(file_path.clone(), prog_sender)?;
                        tx.send(track).unwrap();
                        Ok(())
                    });

                    self.ops_in_progress.push(ThreadTracker::new(
                        new_op_progress_bar,
                        handle,
                        thread_name,
                    ));
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
