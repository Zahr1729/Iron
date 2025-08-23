use std::path::{Path, PathBuf};

use std::fmt::Debug;
use symphonia::core::codecs::CodecParameters;

use crate::common::mipmapchannel::MipMapChannel;

#[derive(Default)]
pub struct Track {
    file_path: Option<PathBuf>,
    file_codec_parameters: CodecParameters,
    length: u64,
    sample_rate: u32,
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
            length: file_codec_parameters.n_frames.unwrap(),
            sample_rate: file_codec_parameters.sample_rate.unwrap(),
            file_codec_parameters,
            file_data_left,
            file_data_right,
        }
    }

    pub fn _file_codec_parameters(&self) -> &CodecParameters {
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

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
