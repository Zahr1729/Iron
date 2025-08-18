use std::any::Any;
use std::sync::Arc;

use crate::common::{dB, track::Track};

pub trait Effect: Send + Sync + Any {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize);
}

impl Effect for Track {
    /// We want this to feedback the useful output slice of data and nothing else - literally just read (and also if it is outside range then 0)
    fn apply(&self, output: &mut [f32], sample_clock: usize, channels: usize) {
        // frame is the instance in time
        for (i, frame) in output.chunks_mut(channels).enumerate() {
            let (left, right) = if i + sample_clock >= self.length() as usize {
                (0.0, 0.0)
            } else {
                (
                    self.sample_data().0[sample_clock + i],
                    self.sample_data().1[sample_clock + i],
                )
            };

            frame[0] = left;
            frame[1] = right;
        }
    }
}

/// Increase/Decrease the volume by the gain in dB.
pub struct Gain {
    gain: dB,
    input: Arc<dyn Effect>,
}

impl Gain {
    pub fn new(gain: dB, input: Arc<dyn Effect>) -> Self {
        Self { gain, input }
    }

    pub fn gain(&self) -> dB {
        self.gain
    }
}

impl Effect for Gain {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.apply(output, start_sample, channels);
        for j in output {
            *j *= self.gain.to_amplitude();
        }
    }
}

pub struct Zero;

impl Effect for Zero {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        for j in output {
            *j = 0.0;
        }
    }
}
