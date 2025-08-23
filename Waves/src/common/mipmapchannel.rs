use std::cmp::Ordering;

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

    /// NOTE STEP MUST BE GIVEN A POWER OF TWO
    /// If data is asked for outside the range needed then just default to zero
    /// Returning true if we split into max / min
    pub fn get_presampled_data_from_step_and_start(
        &self,
        sample_plot_data: &mut SamplePlotData,
    ) -> bool {
        let scope = tracing::trace_span!("get_presampled_data_from_step_and_start");
        let _span = scope.enter();

        if !sample_plot_data.step.is_power_of_two() {
            println!("NOTE step must be a power of 2");
            return false;
        }

        let data_width = sample_plot_data.data[0].len();

        let n = 63 - sample_plot_data.step.leading_zeros() as usize;
        let pyramid_height = self.pyramid_data.len();

        sample_plot_data.is_min_max = n >= self.cutoff_index;

        //println!("{}", n);

        if n >= pyramid_height {
            println!("NOTE log2 of step must not be greater than the pyramid height");
            return false;
        }

        // as we expect the data vector to have the same number of entries in each component (ie for min/max)
        for i in 0..data_width {
            sample_plot_data.data[0][i] = 0.0;
            if sample_plot_data.is_min_max {
                sample_plot_data.data[1][i] = 0.0;
            }
        }

        // get the start sample in the reduced data
        let reduced_start_sample = sample_plot_data.start_sample / sample_plot_data.step;

        for i in 0..data_width {
            // if there is no more data to consider
            if i + reduced_start_sample >= self.pyramid_data[n].len() {
                break;
            }

            // else
            if n < self.cutoff_index {
                sample_plot_data.data[0][i] = self.pyramid_data[n][i + reduced_start_sample];
            } else {
                sample_plot_data.data[0][i] = self.min_pyramid[n][i + reduced_start_sample];
                sample_plot_data.data[1][i] = self.max_pyramid[n][i + reduced_start_sample];
            }
        }

        if n < self.cutoff_index { false } else { true }
    }
}

/// This is a datastructure simply for holding generic plot data in one channel
/// Ie are we drawing the lines straight up, or are we drawing min/max
/// The appropriate data of such
/// And also other stuff like what is our step size, start sample etc.
#[derive(Debug)]
pub struct SamplePlotData {
    pub is_min_max: bool,
    pub start_sample: usize,
    pub step: usize,
    pub data: Vec<Vec<f32>>,
}

impl SamplePlotData {
    pub fn new(step: usize, start_sample: usize, data_width: usize) -> Self {
        Self {
            is_min_max: false,
            start_sample,
            step,
            data: vec![vec![0.0; data_width]; 2],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_power_of_two_fail() {
        let mut vec = vec![0.0; 100];
        for i in 0..100 {
            vec[i] = i as f32;
        }

        let m = MipMapChannel::new(vec, 10);
        let v = m.get_presampled_data_from_step_and_start(5, 3, 20);

        assert_eq!(v.0.len(), 0)
    }

    #[test]
    fn test_power_of_two_succeed() {
        let mut vec = vec![0.0; 100];
        for i in 0..100 {
            vec[i] = i as f32;
        }

        let start = 5;
        let len = 20;

        let m = MipMapChannel::new(vec.clone(), 10);
        let v = m.get_presampled_data_from_step_and_start(start, 4, len);

        for i in 0..len {
            assert_eq!(v.0[0][i], vec[start + i * 4]);
        }
    }

    #[test]
    fn test_power_of_two_overflow() {
        let mut vec = vec![0.0; 100];
        for i in 0..100 {
            vec[i] = i as f32;
        }

        let m = MipMapChannel::new(vec, 10);
        let v = m.get_presampled_data_from_step_and_start(5, 16, 20);
    }
}
