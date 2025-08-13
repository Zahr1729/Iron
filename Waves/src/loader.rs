use std::{path::PathBuf, sync::mpsc};

use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::{CODEC_TYPE_NULL, DecoderOptions},
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

use crate::common::{MipMapChannel, Track};

impl Track {
    pub fn get_data_from_mp3_path(
        file_path: PathBuf,
        update_progress: mpsc::Sender<f32>,
    ) -> Result<Self, Error> {
        // Open the media source.
        let src = std::fs::File::open(&file_path).expect("failed to open media");

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let hint = Hint::new();

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let mut probed =
            symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

        // Get the instantiated format reader.
        let mut format = probed.format;

        println!("META {:?}", probed.metadata.get());

        // Find the first audio track with a known (decodeable) codec.
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(Error::Unsupported("no supported audio tracks"))?;

        let file_codec_parameters = track.codec_params.clone();

        // Use the default options for the decoder.
        let dec_opts: DecoderOptions = Default::default();

        // Create a decoder for the track.
        let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;

        // The decode loop.
        let n_frames = file_codec_parameters.n_frames.unwrap() as usize;
        let mut file_data_left = Vec::with_capacity(n_frames);
        let mut file_data_right = Vec::with_capacity(n_frames);

        let mut prog = 0.0;
        let total = n_frames as f32; // needs to be double for the mipmap

        loop {
            // Get the next packet from the media format.
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(_d)) => {
                    //println!("IO {d}");
                    break;
                    // Seemingly necessary at the end of the loop
                }
                Err(err) => {
                    // A unrecoverable error occured, halt decoding.
                    return Err(err);
                }
            };

            // Consume any new metadata that has been read since the last packet.
            while !format.metadata().is_latest() {
                // Pop the old head of the metadata queue.
                format.metadata().pop();

                // Consume the new metadata at the head of the metadata queue.

                // if let Some(rev) = format.metadata().current() {
                //     // Consume the new metadata at the head of the metadata queue.
                //     println!("META: {:?}", rev);
                // }
            }

            // If the packet does not belong to the selected track, skip over it.
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet into audio samples.
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    match decoded {
                        AudioBufferRef::F32(buf) => {
                            // channel 0 is left channel 1 is right anything else is death.
                            // this stores both of them
                            file_data_left.extend_from_slice(buf.chan(0));
                            file_data_right.extend_from_slice(buf.chan(0));
                            prog += buf.chan(0).len() as f32 / total;

                            update_progress.send(prog).unwrap();
                        }
                        _ => {
                            // Repeat for the different sample formats.
                            unimplemented!()
                        }
                    }
                    // Consume the decoded audio samples (see below).
                }
                Err(Error::IoError(_)) => {
                    // The packet failed to decode due to an IO error, skip the packet.
                    continue;
                }
                Err(Error::DecodeError(_)) => {
                    // The packet failed to decode due to invalid data, skip the packet.
                    continue;
                }
                Err(err) => {
                    // An unrecoverable error occured, halt decoding.
                    return Err(err);
                }
            }
        }

        // Now that we have the generic file data we want to iterate through a few mip map sizes, makes sense to iterate until we get sufficiently far
        let cutoff_index = 5;

        // TRACK HOLDS IMPORTANT METADATA

        Ok(Track::new(
            Some(file_path),
            file_codec_parameters,
            MipMapChannel::new(file_data_left, cutoff_index),
            MipMapChannel::new(file_data_right, cutoff_index),
        ))
    }
}
