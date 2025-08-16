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
}
