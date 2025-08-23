use std::{f32::consts::PI, thread};

use num_complex::{Complex, ComplexFloat};

pub mod mipmapchannel;
pub mod track;

#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct dB(pub f32);

impl dB {
    pub fn to_amplitude(self) -> f32 {
        10.0.powf(self.0 / 20.0)
    }

    pub fn from_amplitude(f: f32) -> Self {
        dB(20.0 * f.log10())
    }
}

#[derive(Clone, Copy)]
pub enum Channel {
    Left,
    Right,
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

    let mut f = vec![Complex { re: 0.0, im: 0.0 }; size];

    let (f0, f1) = f.split_at_mut(size / 2);

    for i in 0..size / 2 {
        s0.push(samples[2 * i]);
        s1.push(samples[2 * i + 1]);
    }

    complex_fft(&s0, f0, inverse);
    complex_fft(&s1, f1, inverse);

    // println!("{:?}", f0);
    // println!("{:?}", f1);

    let angle = 2.0 * PI / size as f32;
    let w = match inverse {
        false => Complex::from_polar(1.0, angle),
        true => Complex::from_polar(1.0, -angle),
    };

    let mut w_i: Complex<f32> = Complex::ONE;

    {
        for i in 0..size / 2 {
            frequencies[i] = f0[i] + w_i * f1[i];
            frequencies[i + size / 2] = f0[i] - w_i * f1[i];

            w_i *= w;
        }
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
