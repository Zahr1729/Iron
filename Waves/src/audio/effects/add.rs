use std::sync::Arc;

use eframe::egui::mutex::Mutex;

use crate::{
    audio::effects::{Effect, EffectError},
    common::mipmapchannel::SamplePlotData,
};

pub struct Add {
    input_0: Mutex<Arc<dyn Effect>>,
    input_1: Mutex<Arc<dyn Effect>>,
}

impl Add {
    pub fn new(input_0: Arc<dyn Effect>, input_1: Arc<dyn Effect>) -> Self {
        Self {
            input_0: Mutex::new(input_0),
            input_1: Mutex::new(input_1),
        }
    }
}

impl Effect for Add {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        //println!("{:?}, {:?}", output, output.len());
        let mut output_1 = vec![0.0; output.len()];
        self.input_0.lock().apply(output, start_sample, channels);
        self.input_1
            .lock()
            .apply(&mut output_1, start_sample, channels);

        //println!("{:?}", output_1);

        for (i, j) in output.iter_mut().zip(output_1) {
            *i += j;
        }
    }

    fn input_count(&self) -> usize {
        2
    }

    fn output_count(&self) -> usize {
        1
    }

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
        match index {
            0 => {
                *self.input_0.lock() = input;
                Ok(())
            }
            1 => {
                *self.input_1.lock() = input;
                Ok(())
            }
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        match index {
            0 => Ok(self.input_0.lock().clone()),
            1 => Ok(self.input_1.lock().clone()),
            _ => Err(EffectError::OutOfBounds(index)),
        }
    }

    fn name(&self) -> &str {
        "Add"
    }

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        //println!("{:?}, {:?}", output, output.len());
        let mut sample_plot_data_1 = SamplePlotData::new(
            sample_plot_data.step,
            sample_plot_data.start_sample,
            sample_plot_data.data[0].len(),
        );
        self.input_0
            .lock()
            .get_waveform_plot_data(sample_plot_data, channel);
        self.input_1
            .lock()
            .get_waveform_plot_data(sample_plot_data, channel);

        //println!("{:?}", output_1);

        for k in 0..sample_plot_data.data.len() {
            for (i, v) in &mut sample_plot_data.data[k].iter_mut().enumerate() {
                *v += sample_plot_data_1.data[k][i];
            }
        }
    }
}
