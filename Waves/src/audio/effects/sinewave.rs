use std::sync::Arc;
use std::{f32::consts::PI, sync::Mutex};

use eframe::egui::Slider;

use crate::audio::effects::{Effect, EffectError};

pub struct SineWave {
    amplitude: Mutex<f32>,
    frequency: Mutex<f32>,
    phase: Mutex<f32>,
}

impl SineWave {
    pub fn new(amplitude: f32, frequency: f32, phase: f32) -> Self {
        Self {
            amplitude: Mutex::new(amplitude),
            frequency: Mutex::new(frequency),
            phase: Mutex::new(phase),
        }
    }
}

impl Effect for SineWave {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        for (i, frame) in output.chunks_mut(channels).enumerate() {
            let v = ((2.0 * PI * (i + start_sample) as f32 / 48000.0)
                * *self.frequency.lock().unwrap()
                - *self.phase.lock().unwrap())
            .sin()
                * *self.amplitude.lock().unwrap();

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

    fn get_waveform_plot_data(
        &self,
        sample_plot_data: &mut crate::common::mipmapchannel::SamplePlotData,
        channel: &crate::common::Channel,
    ) {
        for (j, vec) in &mut sample_plot_data.data.iter_mut().enumerate() {
            for (i, f) in vec.iter_mut().enumerate() {
                let v = ((2.0
                    * PI
                    * (i * sample_plot_data.step + sample_plot_data.start_sample) as f32
                    / 48000.0)
                    * *self.frequency.lock().unwrap()
                    - *self.phase.lock().unwrap())
                .sin()
                    * *self.amplitude.lock().unwrap();

                match j {
                    0 => *f = -v,
                    _ => *f = v,
                }
                *f = v;
            }
        }
    }

    fn data_ui(&self, ui: &mut eframe::egui::Ui, _style: &crate::ui::nodegraph::GraphStyle) {
        ui.add(Slider::new(&mut *self.amplitude.lock().unwrap(), 0.0..=1.0));
        ui.add(Slider::new(
            &mut *self.phase.lock().unwrap(),
            0.0..=2.0 * PI,
        ));
        ui.add(Slider::new(&mut *self.frequency.lock().unwrap(), 20.0..=22000.0).logarithmic(true));
    }
}
