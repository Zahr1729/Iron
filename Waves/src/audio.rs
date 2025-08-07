use crate::common::Track;

use std::{
    sync::{Arc, mpsc},
    thread,
};

use cpal::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// This we want to recieve
#[derive(Debug)]
pub enum AudioCommand {
    /// Perhaps later change Track to something that is "playable", with "playable" meaning that it can find a "next_sample"
    PlayFromSample(Arc<Track>, usize),
    Stop,
}

/// This we want to send
pub enum AudioUpdate {
    AtSample(usize),
}

pub struct AudioThread {
    commands: mpsc::Sender<AudioCommand>,
    updates: mpsc::Receiver<AudioUpdate>,
}

impl AudioThread {
    pub fn new() -> Self {
        let (tx_commands, rx_commands) = mpsc::channel();
        let (tx_updates, rx_updates) = mpsc::channel();

        thread::spawn(move || {
            let host: cpal::Host = cpal::default_host();

            let output_device = host.default_output_device().unwrap();

            let mut current_stream: Option<cpal::Stream> = None;

            loop {
                // Collect new info
                let command = rx_commands.recv().unwrap();

                // Something something

                match command {
                    AudioCommand::Stop => {
                        current_stream.inspect(|stream| stream.pause().unwrap());
                        current_stream = None;
                    }
                    AudioCommand::PlayFromSample(track, start_point) => {
                        let config = output_device.default_output_config().unwrap().config();

                        let buffer_size = track.file_data().0.len();
                        let channels = config.channels as usize;

                        let mut sample_clock: usize = start_point;

                        // We cloning self because this function needs to access (but might outlive the thread (but it won't))
                        let s = track.clone();
                        let mut next_value = move || {
                            sample_clock = sample_clock + 1;

                            if sample_clock >= buffer_size {
                                (0.0, 0.0)
                            } else {
                                (s.file_data().0[sample_clock], s.file_data().1[sample_clock])
                            }
                        };

                        let err_fn = |err| println!("an error occurred on stream: {err}");

                        let stream = output_device
                            .build_output_stream(
                                &config,
                                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                    write_data(data, channels, &mut next_value)
                                },
                                err_fn,
                                None,
                            )
                            .unwrap();
                        stream.play().unwrap();
                        current_stream = Some(stream);

                        fn write_data(
                            output: &mut [f32],
                            channels: usize,
                            // this is the function saying what the next left right data should be
                            next_frame: &mut dyn FnMut() -> (f32, f32),
                        ) {
                            // frame is the instance in time
                            for frame in output.chunks_mut(channels) {
                                let (left, right): (f32, f32) = next_frame();
                                frame[0] = left;
                                frame[1] = right;
                            }
                        }
                    }
                }

                // send updates back.
            }
        });

        Self {
            commands: tx_commands,
            updates: rx_updates,
        }
    }

    pub fn send_command(&self, command: AudioCommand) {
        self.commands.send(command);
    }
}

// loop {
//     // check for updates

//     // apply updates

//     // send updates?
// }
