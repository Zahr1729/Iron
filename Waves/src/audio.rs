use crate::common::Track;

use std::{
    sync::Arc,
    thread,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

impl Track {
    pub fn play_detatched(self: Arc<Self>, host: Arc<cpal::Host>) {
        thread::spawn(move || {
            let output_device = host.default_output_device();

            match output_device {
                None => (),
                Some(device) => {
                    let config = device.default_output_config().unwrap().config();

                    let buffer_size = self.file_data().0.len();
                    let channels = config.channels as usize;

                    // Produce a sinusoid of maximum amplitude.
                    let mut sample_clock: usize = 0;

                    // We cloning self because this function needs to access (but might outlive the thread (but it won't))
                    let s = self.clone();
                    let mut next_value = move || {
                        sample_clock = (sample_clock + 1) % buffer_size;
                        (s.file_data().0[sample_clock], s.file_data().1[sample_clock])
                    };

                    let err_fn = |err| eprintln!("an error occurred on stream: {err}");

                    let stream = device
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

                    std::thread::sleep(std::time::Duration::from_secs_f64(
                        self.file_data().0.len() as f64 / config.sample_rate.0 as f64,
                    ));

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
            };
        });
    }
}

// struct AudioThread{
//  commands       : Sender,
// updates : Receiver
// }
// impl AudioThread{
//     fn new(){
//         tx, rx = channel();

//         thread::spawn(||{
//             rx.get()
//         })

//         Self{
// commands : tx
//         }
//     }
// }

// loop {
//     // check for updates

//     // apply updates

//     // send updates?
// }
