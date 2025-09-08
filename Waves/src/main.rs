use eframe::egui::{self, Button, Color32, Image, Pos2, Rect};
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
    audio::{dag::EffectDAG, effects::zero::Zero},
    common::track::Track,
    player::{AudioThread, AudioUpdate},
    ui::{
        nodegraph::NodeGraph, playpausebutton::PlayPauseButton, progresstracker::ProgressTracker,
        threadtracker::ThreadTracker,
    },
};

struct MyEguiApp {
    node_graph: NodeGraph,
    effect_dag: Arc<EffectDAG>,
    active_track: Option<Arc<Track>>,
    tx_loader: mpsc::Sender<Track>,
    rx_loader: mpsc::Receiver<Track>,
    audio_thread: player::AudioThread,
    current_sample: usize,
    sample_rate: usize,
    is_paused: bool,
    // Must store
    // - widget
    //   - progress bar if in progressed (not completed)
    //   - error message (if panicked)
    // - thread handle, so we can clear out finished ops
    ops_in_progress: Vec<ThreadTracker>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let (tx, rx) = mpsc::channel();

        let mut s = Self {
            node_graph: NodeGraph::new_non_trivial(),
            effect_dag: Arc::new(EffectDAG::new(0, vec![Arc::new(Zero)])),
            tx_loader: tx,
            rx_loader: rx,
            active_track: Default::default(),
            audio_thread: AudioThread::new(),
            ops_in_progress: Default::default(),
            current_sample: 0,
            sample_rate: 48000,
            is_paused: true,
        };

        egui_extras::install_image_loaders(&cc.egui_ctx);

        s.node_graph.audio_data.sample_rate = 48000;

        s
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let scope = tracing::trace_span!("update");
        let _span = scope.enter();

        tracing::info!(tracy.frame_mark = true);

        // force it to update every frame even if nothing is happening
        ctx.request_repaint();

        // Bottom Panel
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // iterate through the relevant buttons.

                ui.horizontal(|ui| {
                    // Return to zero
                    let return_to_zero_button = Image::new(egui::include_image!(
                        r"..\svg\skip-previous-svgrepo-com.svg"
                    ));
                    if ui.button(return_to_zero_button).clicked() {
                        self.current_sample = 0;
                        self.audio_thread
                            .send_command(player::AudioCommand::RelocateTo(
                                self.node_graph.output.clone(),
                                self.current_sample,
                            ));
                    }

                    // Rewind 5 seconds
                    let rewind_button = Image::new(egui::include_image!(
                        r"..\svg\rewind-5-seconds-back-svgrepo-com.svg"
                    ));
                    if ui.button(rewind_button).clicked() {
                        self.current_sample =
                            self.current_sample.saturating_sub(self.sample_rate * 5);
                        self.audio_thread
                            .send_command(player::AudioCommand::RelocateTo(
                                self.node_graph.output.clone(),
                                self.current_sample,
                            ));
                    }

                    // Perhaps group this all inside playpausebutton
                    let play_pause_button = PlayPauseButton::new(self.is_paused);
                    let response = ui.add(play_pause_button);
                    if response.clicked() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
                        match self.is_paused {
                            true => self
                                .audio_thread
                                .send_command(player::AudioCommand::PlayFrom(
                                    self.node_graph.output.clone(),
                                    self.current_sample,
                                )),
                            false => self.audio_thread.send_command(player::AudioCommand::Stop),
                        }
                        self.is_paused = !self.is_paused;
                    }

                    // Fast Forward
                    let fast_forward_button = Image::new(egui::include_image!(
                        r"..\svg\rewind-forward-svgrepo-com.svg"
                    ))
                    .tint(Color32::BLACK);
                    if ui.button(fast_forward_button).clicked() {
                        self.current_sample =
                            self.current_sample.saturating_add(self.sample_rate * 5);
                        self.audio_thread
                            .send_command(player::AudioCommand::RelocateTo(
                                self.node_graph.output.clone(),
                                self.current_sample,
                            ));
                    }
                });
            });

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

            self.node_graph.node_graph_ui(ui);

            while let Ok(update) = self.audio_thread.updates.try_recv() {
                match update {
                    AudioUpdate::CurrentSample(s) => {
                        self.current_sample = s;
                        self.node_graph.audio_data.current_sample = s;
                    }
                }
            }

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx_loader.try_recv() {
                // Go to the beginning to ensure no nasty crashes.
                //self.current_sample = 0;

                self.node_graph.add_track(Arc::new(rx));
            }

            // if let Some(t) = self.active_track.as_ref() {
            //     let waveform_widget =
            //         WaveformWidget::new(t, self.current_sample, self.audio_thread.commands.clone());
            //     ui.add(waveform_widget);

            //     let eq_widget = EQWidget::new(t, 1024, self.current_sample);
            //     ui.add(eq_widget);
            // }

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

use tracing_subscriber::layer::SubscriberExt;

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .expect("setup tracy layer");

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
