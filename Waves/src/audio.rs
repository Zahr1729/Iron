use crate::common::Track;

use std::{
    sync::{Arc, mpsc},
    thread,
};

use cpal::{
    Device, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

/// It is very important that StopAt takes in the position of the cursor as the value is not known by this thread
#[derive(Debug)]
pub enum AudioCommand {
    /// Perhaps later change Track to something that is "playable", with "playable" meaning that it can find a "next_sample"
    PlayFrom(Arc<Track>, usize),
    /// We need the argument for where the pointer is right now
    Stop,
    /// This is specifically for moving the cursor and continues with what it was doing before!
    RelocateTo(Arc<Track>, usize),
}

/// This we want to send back (if anything at all)
#[derive(Debug)]
pub enum AudioUpdate {
    CurrentSample(usize),
}

pub struct AudioThread {
    pub commands: mpsc::Sender<AudioCommand>,
    pub updates: mpsc::Receiver<AudioUpdate>,
}

fn get_stream_from_sample(
    output_device: Device,
    track: Arc<Track>,
    start_point: usize,
    tx: mpsc::Sender<AudioUpdate>,
) -> Stream {
    let config = output_device.default_output_config().unwrap().config();
    let buffer_size = track.sample_data().0.len();
    let channels = config.channels as usize;
    let mut sample_clock: usize = start_point;

    // We cloning self because this function needs to access (but might outlive the thread (but it won't))
    let s = track.clone();
    let mut next_value = move || {
        if sample_clock + 1 >= buffer_size {
            // if we've gone over the limit of the sample just play nothing
            (0.0, 0.0, buffer_size)
        } else {
            sample_clock = sample_clock + 1;
            (
                s.sample_data().0[sample_clock],
                s.sample_data().1[sample_clock],
                sample_clock,
            )
        }
    };

    let err_fn = |err| println!("an error occurred on stream: {err}");

    fn write_data(
        output: &mut [f32],
        channels: usize,
        // this is the function saying what the next left right data should be
        next_frame: &mut dyn FnMut() -> (f32, f32, usize),
        tx: &mpsc::Sender<AudioUpdate>,
    ) {
        let mut latest_sample = 0;

        // frame is the instance in time
        for frame in output.chunks_mut(channels) {
            let (left, right, current_sample) = next_frame();
            frame[0] = left;
            frame[1] = right;
            latest_sample = current_sample;
        }

        tx.send(AudioUpdate::CurrentSample(latest_sample))
            .expect("Channel Closed");
    }

    let stream = output_device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                write_data(data, channels, &mut next_value, &tx)
            },
            err_fn,
            None,
        )
        .unwrap();

    stream
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
                let Ok(command) = rx_commands.recv() else {
                    return;
                };

                // Do stuff with new audio information

                match command {
                    AudioCommand::Stop => {
                        current_stream.inspect(|stream| stream.pause().unwrap());
                        current_stream = None;
                    }
                    AudioCommand::RelocateTo(track, sample) => {
                        if current_stream.is_some() {
                            let new_stream = get_stream_from_sample(
                                output_device.clone(),
                                track,
                                sample,
                                tx_updates.clone(),
                            );
                            new_stream.play().unwrap();
                            current_stream = Some(new_stream);
                        } else {
                            // Also send an update back that its moved only if the cursor is stopped
                            tx_updates
                                .send(AudioUpdate::CurrentSample(sample))
                                .expect("there is no sample to go from!");
                        }
                    }

                    AudioCommand::PlayFrom(track, sample) => {
                        let new_stream = get_stream_from_sample(
                            output_device.clone(),
                            track,
                            sample,
                            tx_updates.clone(),
                        );
                        new_stream.play().unwrap();
                        current_stream = Some(new_stream);
                    }
                }

                // Send relevant updates (currently nothing)
            }
        });

        Self {
            commands: tx_commands,
            updates: rx_updates,
        }
    }

    pub fn send_command(&self, command: AudioCommand) {
        let _ = self.commands.send(command);
    }
}

// loop {
//     // check for updates

//     // apply updates

//     // send updates?
// }
