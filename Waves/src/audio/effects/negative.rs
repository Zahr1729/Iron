use std::sync::{Arc, Mutex};

use eframe::egui::Ui;

use crate::ui::nodegraph::GraphStyle;

use crate::audio::effects::{Effect, EffectError};

/// Increase/Decrease the volume by the gain in dB.
pub struct Negative {
    input: Mutex<Arc<dyn Effect>>,
}

impl Negative {
    pub fn new(input: Arc<dyn Effect>) -> Self {
        Self {
            input: Mutex::new(input),
        }
    }
}

impl Effect for Negative {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize, sample_rate: usize) {
        self.input
            .lock()
            .unwrap()
            .apply(output, start_sample, channels, sample_rate);
        for j in output {
            *j *= -1.0;
        }
    }

    fn input_count(&self) -> usize {
        1
    }

    fn output_count(&self) -> usize {
        1
    }

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
        match index {
            0 => {
                *self.input.lock().unwrap() = input;
                Ok(())
            }
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        match index {
            0 => Ok(self.input.lock().unwrap().clone()),
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn name(&self) -> &str {
        "Negative"
    }

    fn data_ui(&self, _ui: &mut Ui, _style: &GraphStyle) {}

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        self.input
            .lock()
            .unwrap()
            .get_waveform_plot_data(sample_plot_data, channel);

        for v in &mut sample_plot_data.data {
            for j in v {
                *j *= -1.0;
            }
        }
    }
}
