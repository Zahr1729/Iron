use std::sync::{Arc, Mutex};

use eframe::egui::{Slider, Ui};

use crate::common::dB;
use crate::ui::nodegraph::GraphStyle;

use crate::audio::effects::{Effect, EffectError};

/// Increase/Decrease the volume by the gain in dB.
pub struct Gain {
    // State in
    gain: Mutex<dB>,
    input: Mutex<Arc<dyn Effect>>,
}

impl Gain {
    pub fn new(gain: dB, input: Arc<dyn Effect>) -> Self {
        Self {
            gain: Mutex::new(gain),
            input: Mutex::new(input),
        }
    }

    pub fn gain(&self) -> dB {
        *self.gain.lock().unwrap()
    }
}

impl Effect for Gain {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input
            .lock()
            .unwrap()
            .apply(output, start_sample, channels);
        for j in output {
            *j *= self.gain.lock().unwrap().to_amplitude();
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
        "Gain"
    }

    fn data_ui(&self, ui: &mut Ui, _style: &GraphStyle) {
        ui.add(Slider::new(&mut self.gain.lock().unwrap().0, -18.0..=6.0));
    }

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        self.input
            .lock()
            .unwrap()
            .get_waveform_plot_data(sample_plot_data, channel);

        let gain = self.gain.lock().unwrap().to_amplitude();

        for v in &mut sample_plot_data.data {
            for j in v {
                *j *= gain;
            }
        }
    }
}
