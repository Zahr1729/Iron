use std::sync::Arc;

use crate::audio::effects::{Effect, EffectError};

pub struct Zero;

impl Effect for Zero {
    fn apply(&self, output: &mut [f32], _start_sample: usize, _channels: usize) {
        for j in output {
            *j = 0.0;
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
        "Zero"
    }

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        // the sample_plot_data is defaulted to zero so thats fine
        ()
    }
}
