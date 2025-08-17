use eframe::egui;
use symphonia::core::errors::Error;

use std::{
    sync::{Arc, mpsc},
    thread,
};

mod audio;
mod common;
mod loader;
mod player;
mod scene;
mod ui;

use crate::{
    audio::{dag::EffectDAG, effects::Zero},
    common::track::Track,
    player::{AudioThread, AudioUpdate},
    ui::{ProgressTracker, ThreadTracker},
};

struct MyEguiApp {
    effect_dag: Arc<EffectDAG>,
    active_track: Option<Arc<Track>>,
    tx_loader: mpsc::Sender<Track>,
    rx_loader: mpsc::Receiver<Track>,
    audio_thread: player::AudioThread,
    current_sample: usize,
    is_paused: bool,
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
            effect_dag: Arc::new(EffectDAG::new(0, vec![Arc::new(Zero)])),
            tx_loader: tx,
            rx_loader: rx,
            active_track: Default::default(),
            audio_thread: AudioThread::new(),
            ops_in_progress: Default::default(),
            current_sample: 0,
            is_paused: true,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // force it to update every frame even if nothing is happening
        ctx.request_repaint();

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

            while let Ok(update) = self.audio_thread.updates.try_recv() {
                match update {
                    AudioUpdate::CurrentSample(s) => {
                        self.current_sample = s;
                    }
                }
            }

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx_loader.try_recv() {
                // Go to the beginning to ensure no nasty crashes.
                self.current_sample = 0;
                self.active_track = Some(Arc::new(rx));
            }

            if let Some(t) = self.active_track.as_ref() {
                let waveform_widget = ui::WaveformWidget::new(
                    t,
                    self.current_sample,
                    self.audio_thread.commands.clone(),
                );
                ui.add(waveform_widget);

                let eq_widget = ui::EQWidget::new(t, 1024, self.current_sample);
                ui.add(eq_widget);

                // Perhaps group this all inside playpausebutton
                let play_pause_button = ui::PlayPauseButton::new(self.is_paused);
                let response = ui.add(play_pause_button);
                if response.clicked() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
                    match self.is_paused {
                        true => self
                            .audio_thread
                            .send_command(player::AudioCommand::PlayFrom(
                                t.clone(),
                                self.current_sample,
                            )),
                        false => self.audio_thread.send_command(player::AudioCommand::Stop),
                    }
                    self.is_paused = !self.is_paused;
                }
            }

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
                        let track =
                            Track::get_data_from_mp3_path(file_path.clone(), Some(prog_sender))?;
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
    let mut options = eframe::NativeOptions::default();
    //options.viewport.
    options.viewport.drag_and_drop = Some(true);

    // if its native
    let _ = eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );

    // maybe make it web at some point
}
