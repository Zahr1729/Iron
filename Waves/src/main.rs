use eframe::egui::{self};

use eframe::egui::{
    Checkbox, Color32, ComboBox, NumExt as _, Pos2, Response, ScrollArea, Stroke, TextWrapMode,
    Vec2b, WidgetInfo, WidgetType, remap, vec2,
};

use std::default;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread, time,
};
use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    codecs::{CODEC_TYPE_NULL, CodecParameters, DecoderOptions},
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

fn get_data_from_mp3_path(path: PathBuf) -> (Vec<AudioBuffer<f32>>, CodecParameters) {
    // Open the media source.
    let src = std::fs::File::open(&path).expect("failed to open media");

    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    // Create a probe hint using the file's extension. [Optional]
    let hint = Hint::new();

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // Get the instantiated format reader.
    let mut format = probed.format;

    println!("META {:?}", probed.metadata.get());

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    let codec_params = track.codec_params.clone();

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;

    // The decode loop.
    let mut packets = Vec::new();

    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                // The track list has been changed. Re-examine it and create a new set of decoders,
                // then restart the decode loop. This is an advanced feature and it is not
                // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                // for chained OGG physical streams.
                unimplemented!();
            }
            Err(Error::LimitError(d)) => {
                println!("Limit {d}");
                break;
            }
            Err(Error::IoError(d)) => {
                println!("IO {d}");
                break;
                // Seemingly necessary at the end of the loop
            }
            Err(err) => {
                // A unrecoverable error occured, halt decoding.
                panic!("{}", err);
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            format.metadata().pop();

            // Consume the new metadata at the head of the metadata queue.

            if let Some(rev) = format.metadata().current() {
                // Consume the new metadata at the head of the metadata queue.
                println!("META: {:?}", rev);
            }
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => {
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        // channel 0 is left channel 1 is right anything else is death.
                        // this stores both of them
                        packets.push(buf.into_owned());
                    }
                    _ => {
                        // Repeat for the different sample formats.
                        unimplemented!()
                    }
                }
                // Consume the decoded audio samples (see below).
            }
            Err(Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                continue;
            }
            Err(err) => {
                // An unrecoverable error occured, halt decoding.
                panic!("{}", err);
            }
        }
    }

    // TRACK HOLDS IMPORTANT METADATA
    (packets, codec_params)
}

#[derive(PartialEq)]
struct WaveformWidget {
    vertical: bool,
    allow_zoom: Vec2b,
    allow_drag: Vec2b,
    allow_scroll: Vec2b,
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

    for &d in chunk {
        if d.abs() > maxvalue.abs() {
            maxvalue = d;
        }
    }

    maxvalue
}

impl WaveformWidget {
    fn draw_widget(&self, ui: &mut egui::Ui, waveform: &ImportedTrack) -> Response {
        let id = ui.id();
        let step: usize = 1000; // hack

        let range = if let Some(plot_memory) = egui_plot::PlotMemory::load(ui.ctx(), id) {
            plot_memory.bounds().range_x()
        } else {
            0.02..=1000.0
        };

        let time_span = range.end() - range.start();
        let samp_rate = waveform.file_codec_parameters.sample_rate.unwrap() as f64;
        let total_samples_spanned = time_span * samp_rate;
        let step = (total_samples_spanned / 1000.0) as usize + 1;

        let time_per_step = step as f64 / samp_rate;
        let mut chart = egui_plot::BarChart::new(
            format!("{:?}", waveform.file_path),
            waveform
                .file_buffer
                .iter()
                .flat_map(|packet| packet.chan(0).chunks(step).map(get_extreme))
                .enumerate()
                .filter_map(|(x, y)| {
                    let x64 = x as f64 * time_per_step;
                    range.contains(&x64).then(|| (x64, y as f64))
                })
                .map(|(x, y)| egui_plot::Bar::new(x, y).width(time_per_step))
                .collect(),
        )
        .color(Color32::LIGHT_BLUE);

        if !self.vertical {
            chart = chart.horizontal();
        }

        egui_plot::Plot::new("Normal Distribution Demo")
            .legend(egui_plot::Legend::default())
            .clamp_grid(true)
            .allow_zoom(self.allow_zoom)
            .allow_drag(self.allow_drag)
            .allow_scroll(self.allow_scroll)
            .id(id)
            .show(ui, |plot_ui| plot_ui.bar_chart(chart))
            .response
    }
}

struct ImportedTrack {
    file_path: PathBuf,
    file_codec_parameters: CodecParameters,
    file_buffer: Vec<AudioBuffer<f32>>,
}

struct MyEguiApp {
    active: Option<ImportedTrack>,

    tx: mpsc::Sender<ImportedTrack>,
    rx: mpsc::Receiver<ImportedTrack>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let (tx, rx) = mpsc::channel();

        Self {
            tx,
            rx,
            active: Default::default(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // UI
            ui.heading("Hello World!");

            let cd: WaveformWidget = WaveformWidget::default();

            // Take a look at the channel, if theres something new, update the "active file" data
            if let Ok(rx) = self.rx.try_recv() {
                self.active = Some(rx);
            }

            if let Some(lock) = self.active.as_ref() {
                ui.label(format!("{:?}", lock.file_path));
                ui.label(format!("{:?}", lock.file_codec_parameters));

                cd.draw_widget(ui, lock);
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
                    let tx = self.tx.clone();

                    thread::spawn(move || {
                        let (file_buffer, file_codec_parameters) =
                            get_data_from_mp3_path(file_path.clone());
                        let track = ImportedTrack {
                            file_buffer,
                            file_codec_parameters,
                            file_path,
                        };

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
