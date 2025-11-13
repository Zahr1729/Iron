use std::f32::INFINITY;
use std::sync::{Arc, Mutex};

use eframe::egui::{Slider, Ui};

use crate::common::dB;
use crate::ui::nodegraph::GraphStyle;

use crate::audio::effects::{Effect, EffectError};

/// Change.
pub struct Compression {
    // State in
    attack: Mutex<f32>,
    threshold: Mutex<dB>,
    above_ratio: Mutex<f32>,
    // below_ratio: Mutex<f32>,
    input: Mutex<Arc<dyn Effect>>,
}

impl Compression {
    pub fn new(attack: f32, threshold: dB, above: f32, below: f32, input: Arc<dyn Effect>) -> Self {
        Self {
            attack: Mutex::<f32>::new(attack),
            threshold: Mutex::<dB>::new(threshold),
            above_ratio: Mutex::<f32>::new(above),
            // below_ratio: Mutex::<f32>::new(below),
            input: Mutex::<Arc<dyn Effect>>::new(input),
        }
    }
}

impl Effect for Compression {
    /// This doesnt properly work
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input
            .lock()
            .unwrap()
            .apply(output, start_sample, channels);

        let threshold_db = *self.threshold.lock().unwrap();
        let max_db = dB::from_amplitude(
            output
                .iter()
                .map(|f| f.abs())
                .reduce(f32::max)
                .unwrap_or(0.),
        );
        let difference_db = max_db - threshold_db;
        let above_multiplier = 1.0 - *self.above_ratio.lock().unwrap();
        // let below_multiplier = *self.below_ratio.lock().unwrap();
        for j in output {
            let parity = if j.is_sign_positive() { 1.0 } else { -1.0 };
            let input_db = dB::from_amplitude(j.abs());
            *j = if max_db > threshold_db {
                *j * (1.0 + above_multiplier * difference_db.0 / threshold_db.0)
            } else {
                *j
            };
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
        "Comp."
    }

    fn data_ui(&self, ui: &mut Ui, _style: &GraphStyle) {
        ui.add(Slider::new(&mut *self.attack.lock().unwrap(), 1.0..=100.0).logarithmic(true));
        ui.add(Slider::new(
            &mut self.threshold.lock().unwrap().0,
            -30.0..=0.0,
        ));
        ui.add(Slider::new(
            &mut *self.above_ratio.lock().unwrap(),
            0.0..=1.0,
        ));
        // ui.add(Slider::new(
        //     &mut *self.below_ratio.lock().unwrap(),
        //     0.0..=1.0,
        // ));
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

        let threshold_db = *self.threshold.lock().unwrap();

        for v in &mut sample_plot_data.data {
            let max_db =
                dB::from_amplitude(v.iter().map(|f| f.abs()).reduce(f32::max).unwrap_or(0.));
            let difference_db = max_db - threshold_db;
            let above_multiplier = 1.0 - *self.above_ratio.lock().unwrap();
            for j in v {
                let parity = if j.is_sign_positive() { 1.0 } else { -1.0 };
                let input_db = dB::from_amplitude(j.abs());
                *j = if max_db > threshold_db {
                    *j * (1.0 + above_multiplier * difference_db.0 / threshold_db.0)
                } else {
                    *j
                };
            }
        }
    }
}
