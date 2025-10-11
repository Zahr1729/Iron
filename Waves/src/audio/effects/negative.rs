use std::sync::Arc;

use eframe::egui::mutex::Mutex;
use eframe::egui::{Slider, Ui};

use crate::common::dB;
use crate::ui::eqwidget::EQWidget;
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
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.lock().apply(output, start_sample, channels);
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
            .get_waveform_plot_data(sample_plot_data, channel);

        for v in &mut sample_plot_data.data {
            for j in v {
                *j *= -1.0;
            }
        }
    }
}
