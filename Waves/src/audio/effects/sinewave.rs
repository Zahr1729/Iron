use std::f32::consts::PI;
use std::sync::Arc;

use crate::audio::effects::{Effect, EffectError};

pub struct SineWave {
    amplitude: f32,
    frequency: f32,
    phase: f32,
}

impl SineWave {
    pub fn new(amplitude: f32, frequency: f32, phase: f32) -> Self {
        Self {
            amplitude,
            frequency,
            phase,
        }
    }
}

impl Effect for SineWave {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        for (i, frame) in output.chunks_mut(channels).enumerate() {
            let v = ((2.0 * PI * (i + start_sample) as f32 / 48000.0) * self.frequency
                - self.phase)
                .sin()
                * self.amplitude;

            for f in frame {
                *f = v;
            }
        }
    }

    fn input_count(&self) -> usize {
        0
    }

    fn output_count(&self) -> usize {
        1
    }

    fn set_input_at_index(&self, index: usize, _input: Arc<dyn Effect>) -> Result<(), EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn name(&self) -> &str {
        "Sine Wave"
    }
}
