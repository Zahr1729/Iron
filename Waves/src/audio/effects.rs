use std::cell::UnsafeCell;
use std::f32::consts::PI;
use std::fmt::Debug;
use std::ops::SubAssign;
use std::sync::Arc;
use std::{any::Any, sync::atomic::AtomicPtr};

use eframe::egui::Ui;
use eframe::egui::mutex::Mutex;

use crate::common::{dB, track::Track};
use crate::ui::eqwidget::EQWidget;
use crate::ui::waveformwidget::WaveformWidget;

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

    fn draw(&self, ui: &mut Ui, current_sample: usize, sample_rate: u32) {
        // do a eq diagram
        let data_width = 1024;
        let start_sample = (current_sample).saturating_sub((data_width) / 2);
        let mut sample_data = vec![0.0; data_width];
        self.apply(&mut sample_data, start_sample, 1);
        let eq_widget = EQWidget::new(sample_data, sample_rate);
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

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
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

/// Increase/Decrease the volume by the gain in dB.
pub struct Gain {
    gain: dB,
    input: Mutex<Arc<dyn Effect>>,
}

impl Gain {
    pub fn new(gain: dB, input: Arc<dyn Effect>) -> Self {
        Self {
            gain,
            input: Mutex::new(input),
        }
    }

    pub fn gain(&self) -> dB {
        self.gain
    }
}

impl Effect for Gain {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.lock().apply(output, start_sample, channels);
        for j in output {
            *j *= self.gain.to_amplitude();
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
}

pub struct Zero;

impl Effect for Zero {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
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

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn name(&self) -> &str {
        "Zero"
    }
}

pub struct Output {
    input: Mutex<Arc<dyn Effect>>,
}

impl Output {
    pub fn new(input: Arc<dyn Effect>) -> Self {
        Self {
            input: Mutex::new(input),
        }
    }
}

impl Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output").finish()
    }
}

impl Effect for Output {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.input.lock().apply(output, start_sample, channels);
    }

    fn input_count(&self) -> usize {
        1
    }

    fn output_count(&self) -> usize {
        0
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
        "Output"
    }
}

pub struct SineWave {
    amplitude: f32,
    frequency: f32,
    phase: f32,
}

impl SineWave {
    pub fn new(amplitude: f32, frequency: f32, phase: f32) -> Self {
        Self {
            amplitude,
            frequency,
            phase,
        }
    }
}

impl Effect for SineWave {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        for (i, frame) in output.chunks_mut(channels).enumerate() {
            let v = (((2.0 * PI * (i + start_sample) as f32) * self.frequency / 48000.0)
                - self.phase)
                .sin()
                * self.amplitude;

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

    fn set_input_at_index(&self, index: usize, input: Arc<dyn Effect>) -> Result<(), EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn get_input_at_index(&self, index: usize) -> Result<Arc<dyn Effect>, EffectError> {
        Err(EffectError::OutOfBounds(index))
    }

    fn name(&self) -> &str {
        "Sine Wave"
    }
}

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
}
