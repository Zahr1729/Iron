use eframe::egui::Ui;
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::common::track::Track;
use crate::ui::eqwidget::EQWidget;
use crate::ui::nodegraph::GraphStyle;

pub mod add;
pub mod gain;
pub mod output;
pub mod sinewave;
pub mod zero;

#[derive(Debug)]
pub enum EffectError {
    OutOfBounds(usize),
}

pub trait Effect: Send + Sync + Any {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize);
    fn input_count(&self) -> usize;
    fn output_count(&self) -> usize;
    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError>;
    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError>;
    fn name(&self) -> &str;

    fn data_ui(&self, _ui: &mut Ui, _style: &GraphStyle) {
        ()
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
        let mut sample_data = vec![0.0; data_width];
        self.apply(&mut sample_data, start_sample, 1);
        let eq_widget = EQWidget::new(sample_data, sample_rate, plot_size);
        ui.add(eq_widget);
    }
}

impl Effect for Track {
    /// We want this to feedback the useful output slice of data and nothing else - literally just read (and also if it is outside range then 0)
    fn apply(&self, output: &mut [f32], sample_clock: usize, channels: usize) {
        // frame is the instance in time
        for (i, frame) in output.chunks_mut(channels).enumerate() {
            let (left, right) = if i + sample_clock >= self.length() as usize {
                (0.0, 0.0)
            } else {
                (
                    self.sample_data().0[sample_clock + i],
                    self.sample_data().1[sample_clock + i],
                )
            };

            // for stereo
            if channels == 2 {
                frame[1] = right;
            }

            // if only want mono
            frame[0] = left;
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
        "Track"
    }

    // fn draw(&self, ui: &mut Ui, start_sample: usize, sample_rate: u32) {
    //     let binding = Arc::new(self.clone());
    //     let waveform_widget = WaveformWidget::new(&binding, start_sample, None);
    //     ui.add(waveform_widget);
    // }
}
