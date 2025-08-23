use std::sync::Arc;

use eframe::egui::mutex::Mutex;
use eframe::egui::{Slider, Ui};

use crate::common::dB;
use crate::ui::eqwidget::EQWidget;
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
        *self.gain.lock()
    }
}

impl Effect for Gain {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.lock().apply(output, start_sample, channels);
        for j in output {
            *j *= self.gain.lock().to_amplitude();
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
        "Gain"
    }

    fn data_ui(&self, ui: &mut Ui, _style: &GraphStyle) {
        ui.add(Slider::new(&mut self.gain.lock().0, -18.0..=6.0));
    }

    fn draw_plot(
        &self,
        ui: &mut Ui,
        current_sample: usize,
        sample_rate: u32,
        plot_size: (f32, f32),
    ) {
        // do a eq diagram
        let data_width = 1024;
        let start_sample = (current_sample).saturating_sub((data_width) / 2);
        let mut sample_data = std::vec![0.0; data_width];
        self.apply(&mut sample_data, start_sample, 1);
        let eq_widget = EQWidget::new(sample_data, sample_rate, plot_size);
        ui.add(eq_widget);
    }
}
