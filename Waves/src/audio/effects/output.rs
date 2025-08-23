use std::fmt::Debug;
use std::sync::Arc;

use eframe::egui::mutex::Mutex;

use crate::audio::effects::{Effect, EffectError};

pub struct Output {
    input: Mutex<Arc<dyn Effect>>,
}

impl Output {
    pub fn new(input: Arc<dyn Effect>) -> Self {
        Self {
            input: Mutex::new(input),
        }
    }
}

impl Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output").finish()
    }
}

impl Effect for Output {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.lock().apply(output, start_sample, channels);
    }

    fn input_count(&self) -> usize {
        1
    }

    fn output_count(&self) -> usize {
        0
    }

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
        match index {
            0 => {
                *self.input.lock() = input;
                Ok(())
            }
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        match index {
            0 => Ok(self.input.lock().clone()),
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn name(&self) -> &str {
        "Output"
    }

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        self.input
            .lock()
            .get_waveform_plot_data(sample_plot_data, channel);
    }
}
