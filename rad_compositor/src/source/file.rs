use std::mem;
use std::path::Path;
use std::{fs::File, path::PathBuf};

use symphonia::core::{audio::SampleBuffer, formats::FormatReader};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::utils::SampleBuf;
use super::BaseSource;

pub struct FileSource {
    path: PathBuf,
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    sample_rate: u32,
    channels: u16,
    track_id: u32,
    cache: SampleBuf,
    buf: SampleBuf,
    duration: usize
}

impl FileSource {
    pub fn new(path: PathBuf) -> Option<Self> {
        let file = File::open(&path).unwrap();

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let mut hint = Hint::default();
        hint.mime_type("audio/mpeg");

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let probed;
        if let Ok(_probed) = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts) {
            
            probed = _probed
        } else {
            // TODO: Add error handling
            return None;
            // return Err(InitError::UnsupportedFormat);
        }

        // Get the instantiated format reader.
        let mut format = probed.format;

        // Find the first audio track with a known (decodable) codec.
        let track;
        if let Some(_track) = format.default_track() {
            track = _track.clone();
        } else {
            // TODO: Add error handling
            return None;
            // return Err(InitError::NoTrackFound);
        }

        let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).unwrap();

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;        

        let decoded = decoder.decode(&format.next_packet().unwrap()).unwrap();
        let mut buf: SampleBuffer<f32> = SampleBuffer::new(decoded.capacity() as u64, decoded.spec().clone());
        let spec = decoded.spec().clone();
        buf.copy_interleaved_ref(decoded);
        
        Some(FileSource {
            decoder,
            path,
            reader: format,
            sample_rate: track.codec_params.sample_rate.unwrap(),
            channels: spec.channels.count() as u16,
            track_id,
            cache: SampleBuf::new(0, Vec::new()),
            buf: SampleBuf::new(0, buf.samples().to_vec()),
            duration: track.codec_params.n_frames.unwrap() as usize,
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl BaseSource for FileSource {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn current_duration_frames(&self) -> usize {
        self.duration
    }

    fn duration(&self) -> Option<usize> {
        Some(self.duration)
    }

    fn get_by_frame_i(&mut self, frame_i: usize) -> Option<Vec<super::TSample>> {
        let buf_size_in_frames = self.buf.len() / self.channels as usize;
        let buf_end_i = self.buf.start() + buf_size_in_frames;
        
        if self.buf.start() <= frame_i && frame_i < buf_end_i {
            // Start index of the frame in the buffer
            let buf_i = (frame_i - self.buf.start()) * self.channels as usize;

            return Some(
                self.buf.samples[buf_i..buf_i + self.channels as usize].to_vec()
            );
        }

        if self.cache.start() <= frame_i && frame_i < self.buf.start() {
            // Start index of the frame in the buffer
            let buf_i = (frame_i - self.cache.start()) * self.channels as usize;

            return Some(
                self.cache.samples[buf_i..buf_i + self.channels as usize].to_vec()
            );
        }

        if buf_end_i + buf_size_in_frames - 1 < frame_i || frame_i < self.cache.start() {
            let seek_time = SeekTo::TimeStamp { ts: frame_i as u64, track_id: self.track_id };
            self.reader.seek(SeekMode::Accurate, seek_time).ok()?;
        }

        let next_packet = self.reader.next_packet().ok()?;
        let decoded = self.decoder.decode(&next_packet).ok()?;

        let buf = SampleBuf::from_audio_buf_ref(next_packet.ts as usize, &decoded);
        let last_buf = mem::replace(&mut self.buf, buf);
        self.cache = last_buf;

        self.get_by_frame_i(frame_i)
    }
}