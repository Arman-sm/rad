// https://github.com/pdeljanov/Symphonia/blob/master/symphonia/examples/getting-started.rs

use std::fs::File;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::source::{utils::delay_iter, iter::IterSrc};

#[derive(Debug)]
pub enum InitError {
	UnsupportedCodec,
    UnsupportedFormat,
    NoTrackFound
}

pub fn init_symphonia_src(file: File, suggested_mime_type: &str) -> Result<IterSrc, InitError> {
    // Create the media source stream.
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a probe hint using the file's extension. [Optional]
    let mut hint = Hint::new();
    hint.mime_type(suggested_mime_type);

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed;
    if let Ok(_probed) = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts) {
        
        probed = _probed
    } else {
        return Err(InitError::UnsupportedFormat);
    }

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodable) codec.
    let track;
    if let Some(_track) = format.default_track() {
        track = _track;
    } else {
        return Err(InitError::NoTrackFound);
    }

    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).unwrap();

    // Store the track identifier, it will be used to filter packets.
    // let track_id = track.id;

    // The decode loop.
    let mut func = move || { loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => {return None;}
        };

        use symphonia::core::errors::Error;
        match decoder.decode(&packet) {
            Ok(_decoded) => {
                let mut buf: SampleBuffer<f32> = SampleBuffer::new(_decoded.capacity() as u64, _decoded.spec().clone());
                let spec = _decoded.spec().clone();
                buf.copy_interleaved_ref(_decoded);

                return Some((buf, spec));
            }
            Err(Error::IoError(_)) => {continue;}
            Err(Error::DecodeError(_)) => {continue;}
            Err(err) => panic!("{}", err)
        }
    }};

    let (buf, spec) = func().unwrap();

    let first_buf = buf.samples().to_vec();
    Ok(
        IterSrc::new(
        delay_iter(
            Box::new(move || {
                if let Some((buf, _)) = func() {
                    return Some(buf.samples().to_vec());
                }

                None
            }
        ), Some(first_buf)),
        spec.rate, spec.channels.count() as usize)
    )
}