use std::path::{Path, PathBuf};

use symphonia::core::codecs::CodecParameters;

#[derive(Default, Clone)]
pub struct Track {
    file_path: Option<PathBuf>,
    file_codec_parameters: CodecParameters,
    file_data_left: Vec<f32>,
    file_data_right: Vec<f32>,
}

impl Track {
    pub fn new(
        file_path: Option<PathBuf>,
        file_codec_parameters: CodecParameters,
        file_data_left: Vec<f32>,
        file_data_right: Vec<f32>,
    ) -> Track {
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

    pub fn file_data(&self) -> (&[f32], &[f32]) {
        (&self.file_data_left, &self.file_data_right)
    }

    pub fn file_path(&self) -> Option<&Path> {
        match &self.file_path {
            None => None,
            Some(path) => Some(&path),
        }
    }
}
