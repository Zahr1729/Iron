use std::{
    cmp::Ordering,
    f32::consts::PI,
    path::{Path, PathBuf},
};

use num_complex::{Complex, ComplexFloat};
use std::fmt::Debug;
use symphonia::core::codecs::CodecParameters;

pub enum Channel {
    Left,
    Right,
}

#[derive(Default, Clone)]
pub struct MipMapChannel {
    pyramid_data: Vec<Vec<f32>>,
    max_pyramid: Vec<Vec<f32>>,
    min_pyramid: Vec<Vec<f32>>,
    cutoff_index: usize,
}

impl MipMapChannel {
    fn resample_data_with_comparison(
        data: Vec<f32>,
        func: impl Fn(&f32, &f32) -> Ordering,
    ) -> Vec<f32> {
        data.chunks(2)
            .map(|c| *c.iter().max_by(|x, y| func(*x, *y)).unwrap())
            .collect::<Vec<f32>>()
    }

    pub fn new(data: Vec<f32>, cutoff_index: usize) -> Self {
        let mut i = 0;
        let mut size = data.len() / 2;
        let mut pyramid_data = vec![data];
        let mut max_pyramid = pyramid_data.clone();
        let mut min_pyramid = pyramid_data.clone();
        while size > 1000 {
            // Fix up a better comparison at some point
            let normal = MipMapChannel::resample_data_with_comparison(
                pyramid_data[i].clone(),
                |x: &f32, y: &f32| {
                    x.abs()
                        .partial_cmp(&y.abs())
                        .expect("samples cannot be NaN")
                },
            );
            pyramid_data.push(normal);

            let max = MipMapChannel::resample_data_with_comparison(
                max_pyramid[i].clone(),
                |x: &f32, y: &f32| x.partial_cmp(&y).expect("samples cannot be NaN"),
            );
            max_pyramid.push(max);

            let min = MipMapChannel::resample_data_with_comparison(
                min_pyramid[i].clone(),
                |x: &f32, y: &f32| (-x).partial_cmp(&(-y)).expect("samples cannot be NaN"),
            );
            min_pyramid.push(min);

            i += 1;
            size /= 2;
        }

        Self {
            pyramid_data,
            max_pyramid,
            min_pyramid,
            cutoff_index,
        }
    }
    pub fn get_full_data(&self) -> &[f32] {
        &self.pyramid_data[0]
    }

    /// Returns minmap array of appropriate size, intended stepsize for each sample and a float indicating how big step size optimally should be
    /// Returns either one or two data sets being a line or the 'min' and 'max' lines
    pub fn get_presampled_data_and_step(&self, sample_range: usize) -> (Vec<&[f32]>, usize, f32) {
        let pyramid_height = self.pyramid_data.len();
        let f = pyramid_height as f32
            - ((self.get_full_data().len() as f32 + 0.1).log2()
                - (sample_range as f32 + 0.1).log2()
                + 1.0)
                .clamp(0.0, pyramid_height as f32);
        let n = f as usize;

        if n < self.cutoff_index {
            (vec![&self.pyramid_data[n]], 1 << n, f)
        } else {
            (vec![&self.min_pyramid[n], &self.max_pyramid[n]], 1 << n, f)
        }
    }
}

#[derive(Default, Clone)]
pub struct Track {
    file_path: Option<PathBuf>,
    file_codec_parameters: CodecParameters,
    file_data_left: MipMapChannel,
    file_data_right: MipMapChannel,
}

impl Debug for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Track")
            .field("file_path", &self.file_path)
            .field("file_codec_parameters", &self.file_codec_parameters)
            .finish()
    }
}

impl Track {
    pub fn new(
        file_path: Option<PathBuf>,
        file_codec_parameters: CodecParameters,
        file_data_left: MipMapChannel,
        file_data_right: MipMapChannel,
    ) -> Self {
        Self {
            file_path,
            file_codec_parameters,
            file_data_left,
            file_data_right,
        }
    }

    pub fn file_codec_parameters(&self) -> &CodecParameters {
        &self.file_codec_parameters
    }

    pub fn sample_data(&self) -> (&[f32], &[f32]) {
        (
            self.file_data_left.get_full_data(),
            self.file_data_right.get_full_data(),
        )
    }

    //pub fn mipmap_file_data(&self) -> (&[f32], &[f32]) {}

    pub fn _file_path(&self) -> Option<&Path> {
        match &self.file_path {
            None => None,
            Some(path) => Some(&path),
        }
    }

    pub fn file_data_left(&self) -> &MipMapChannel {
        &self.file_data_left
    }

    pub fn file_data_right(&self) -> &MipMapChannel {
        &self.file_data_right
    }
}

/// This is the maths involving complex numbers doing the actual computations
/// Fix this up by passing an iterator for samples along with the size.
fn complex_fft(samples: &[Complex<f32>], frequencies: &mut [Complex<f32>], inverse: bool) {
    let size = samples.len();
    if !size.is_power_of_two() {
        println!("Power of 2 samples not provided!");
        return;
    }

    if size == 1 {
        frequencies[0] = samples[0];
        return;
    }

    // Assuming the samples is evenly sized we can apply this function recursively
    let mut s0 = Vec::with_capacity(size / 2);
    let mut s1 = Vec::with_capacity(size / 2);

    let mut f0 = vec![Complex { re: 0.0, im: 0.0 }; size / 2];
    let mut f1 = vec![Complex { re: 0.0, im: 0.0 }; size / 2];
    for i in 0..size / 2 {
        s0.push(samples[2 * i]);
        s1.push(samples[2 * i + 1]);
    }

    complex_fft(&s0, &mut f0, inverse);
    complex_fft(&s1, &mut f1, inverse);

    // println!("{:?}", f0);
    // println!("{:?}", f1);

    let angle = 2.0 * PI / size as f32;
    let w = match inverse {
        false => Complex::from_polar(1.0, angle),
        true => Complex::from_polar(1.0, -angle),
    };

    let mut w_i: Complex<f32> = Complex::ONE;

    for i in 0..size / 2 {
        frequencies[i] = f0[i] + w_i * f1[i];
        frequencies[i + size / 2] = f0[i] - w_i * f1[i];

        w_i *= w;
    }
}

/// This should always take in an array of size 2^n, typically 4096
/// The sampling rate we expect is 48kHz and so this should give us 10db bands with the frequency info
/// Can make it more dense
/// Want to overwrite the data in frequencies so we don't waste
pub fn fft(samples: &[f32]) -> Vec<f32> {
    let cmpx_samples: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();
    let mut cmpx_freqs: Vec<Complex<f32>> = samples
        .iter()
        .map(|&_s| Complex { re: 0.0, im: 0.0 })
        .collect();

    complex_fft(&cmpx_samples, &mut cmpx_freqs, false);

    let sqrt_n = (cmpx_samples.len() as f32).sqrt();

    let freqs = &cmpx_freqs
        .iter()
        .map(|c| (c.abs() / sqrt_n))
        .collect::<Vec<f32>>();

    freqs.clone().to_vec()
}

#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use rand::Rng;

    use super::*;

    #[test]
    fn test_average_to_zero() {
        let n = 4096;
        for i in 0..100 {
            let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(i);
            let samples = (0..n)
                .map(|_x| (rng.random::<f32>() - 0.5))
                .collect::<Vec<_>>();

            // let n = 4;
            // let samples = [1.0, -1.0, 1.0, -1.0]
            //     .iter()
            //     .map(|x| Complex { re: *x, im: 0.0 })
            //     .collect::<Vec<_>>();

            //let mut freqs = vec![Complex::ZERO; n];

            let freqs = fft(&samples);

            let mean = samples.iter().sum::<f32>() / (n as f32).sqrt();

            // println!("{:?}, \n \n {:?}", samples, inv);

            // println!("{:?}, {:?}", freqs[0], mean);

            assert!(
                (freqs[0] - mean.abs()).abs() < 0.2,
                "Mean does not match the zero frequency average"
            );
        }
    }

    #[test]
    fn test_complex_inverse() {
        let n = 4096;
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(12);
        let samples = (0..n)
            .map(|_x| Complex {
                re: rng.random(),
                im: 0.0,
            })
            .collect::<Vec<_>>();

        // let n = 4;
        // let samples = [1.0, -1.0, 1.0, -1.0]
        //     .iter()
        //     .map(|x| Complex { re: *x, im: 0.0 })
        //     .collect::<Vec<_>>();

        let mut freqs = vec![Complex::ZERO; n];

        complex_fft(&samples, &mut freqs, false);

        // Inverse
        let mut inv = vec![Complex::ZERO; n];

        complex_fft(&freqs, &mut inv, true);

        //println!(freqs)

        // println!("{:?}, \n \n {:?}", samples, inv);

        for i in 0..n {
            assert!(
                (samples[i] - (inv[i] / n as f32)).abs() < 0.0002,
                "double fourier transform does not give identity: {i}"
            );
        }
    }

    #[test]
    fn test_silence() {
        let samples = [0.0; 100];

        let freqs = fft(&samples);

        // What should happen when all zeros?
        // no freqs?
        assert!(
            freqs.iter().all(|&f| f == 0.0),
            "Non-zero frequency found for zero input data"
        )
    }

    #[test]
    fn test_base_case() {
        let samples = vec![0.5];

        let freq = fft(&samples);

        assert_eq!(freq[0], 0.5, "base case fails");
    }

    #[test]
    fn test_highest_freq() {
        let samples = vec![1.0, -1.0, 1.0, -1.0];

        let freqs = fft(&samples);
        println!("{:?}", freqs);

        for i in 0..4 {
            match i {
                2 => assert!(freqs[i].abs() > 0.5, "highest frequency not detected"),
                _ => assert!(freqs[i].abs() < 0.005, "non highest frequencies detected"),
            }
        }
    }

    #[test]
    fn test_small_offset() {
        let samples = vec![1.0 + 0.5, -1.0 + 0.5, 1.0 + 0.5, -1.0 + 0.5];

        let freqs = fft(&samples);

        // base is sum of average of all samples
        assert_eq!(freqs, [1.0, 0.0, 2.0, 0.0], "base case fails");
        println!("{:?}", freqs);
    }

    #[test]
    fn test_8() {
        let samples_0 = [1.0, -1.0].repeat(4);
        let samples_1 = [1.0, 0.0, -1.0, 0.0].repeat(2);

        let samples = samples_0
            .into_iter()
            .zip(samples_1.into_iter())
            .map(|(x, y)| x + y)
            .collect::<Vec<_>>();

        let freqs = fft(&samples);

        assert_eq!(
            freqs,
            [0.0, 0.0, 1.4142135, 0.0, 2.828427, 0.0, 1.4142135, 0.0],
            "sum of two sine waves do not react accordingly"
        )
    }

    #[test]
    fn test_sine() {
        let samples = (0..4096)
            .map(|x| (((x as f32) / 48000.0) * 2.0 * PI * 440.0).sin())
            .collect::<Vec<_>>();

        let freqs = fft(&samples);

        // println!("{:?}", samples);
        // println!("{:?}", freqs);

        // Due to sampling rates 35-40 should have the first peak

        for i in 0..samples.len() {
            match i {
                35..=40 => assert!(
                    freqs[i].abs() >= 1.0,
                    "440Hz frequency found to be near zero {i}"
                ),
                4055..=4061 => assert!(
                    freqs[i].abs() >= 1.0,
                    "440Hz frequency found to be near zero {i}"
                ),
                _ => assert!(
                    freqs[i].abs() <= 4.0,
                    "Frequencies away from 440Hz not zero {i}"
                ),
            }
        }
    }
}
