use eframe::egui::Ui;
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::common::Channel;
use crate::common::mipmapchannel::SamplePlotData;
use crate::common::track::Track;
use crate::ui::eqwidget::EQWidget;
use crate::ui::nodegraph::GraphStyle;
use crate::ui::waveformwidget::WaveformWidget;

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

    fn get_waveform_plot_data(&self, sample_plot_data: &mut SamplePlotData, channel: &Channel);

    fn data_ui(&self, _ui: &mut Ui, _style: &GraphStyle) {
        ()
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

    fn get_waveform_plot_data(&self, sample_plot_data: &mut SamplePlotData, channel: &Channel) {
        let scope = tracing::trace_span!("track.get_plot_data");
        let _span = scope.enter();

        //println!("{sample_plot_data:?}");

        match channel {
            Channel::Left => self
                .file_data_left()
                .get_presampled_data_from_step_and_start(sample_plot_data),
            Channel::Right => self
                .file_data_right()
                .get_presampled_data_from_step_and_start(sample_plot_data),
        };

        //println!("{:?}", sample_plot_data);
    }

    // For the track we want to render the waveform its

    // fn draw(&self, ui: &mut Ui, start_sample: usize, sample_rate: u32) {
    //     let binding = Arc::new(self.clone());
    //     let waveform_widget = WaveformWidget::new(&binding, start_sample, None);
    //     ui.add(waveform_widget);
    // }
}
