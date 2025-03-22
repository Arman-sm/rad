use std::path::PathBuf;

use rad_storage::{respond_storage, GLOBAL_SEGMENT_STORE};
use rad_storage::segment_store::PileID;
use symphonia::core::{audio::SampleBuffer, formats::FormatReader};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::utils::sample_buf::SampleBuf;
use super::{BaseSource, TFrameIdx};

pub enum StreamOrigin {
    FileSystem(PathBuf),
    RemoteClient
}

/// A source type dedicated to reading and deserializing compressed and formatted audio from an stream (network, file, ...)
/// 
/// All streams that implement `io::Read` and are sync/send are able to be fed into the source.
pub struct FormattedStreamSource {
    storage_pile_id: PileID,
    origin: Option<StreamOrigin>,
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    sample_rate: TFrameIdx,
    channels: u8,
    track_id: u32,
    last_frame_idx: TFrameIdx,
    duration: TFrameIdx,
}

impl FormattedStreamSource {
    pub fn open_path(path: PathBuf) -> Option<Self> {
        let file = std::fs::File::open(&path).unwrap();
        let origin = StreamOrigin::FileSystem(path);

        Self::open_stream(Box::new(file), Some(origin))
    }
    
    /// Warning: The stream must yield something on the first opening
    pub fn open_stream(stream: Box<dyn MediaSource>, origin: Option<StreamOrigin>) -> Option<Self> {
        // Create the media source stream.
        let mss = MediaSourceStream::new(stream, Default::default());

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
        
        let pile_id = GLOBAL_SEGMENT_STORE.write().unwrap().new_pile_id();

        Some(FormattedStreamSource {
            storage_pile_id: pile_id,
            decoder,
            origin,
            reader: format,
            sample_rate: track.codec_params.sample_rate.unwrap() as TFrameIdx,
            channels: spec.channels.count() as u8,
            track_id,
            duration: track.codec_params.n_frames.unwrap() as TFrameIdx,
            last_frame_idx: 0
        })
    }

    pub fn origin(&self) -> &Option<StreamOrigin> {
        &self.origin
    }
}

impl BaseSource for FormattedStreamSource {
    fn sample_rate(&self) -> TFrameIdx {
        self.sample_rate
    }

    fn current_duration_frames(&self) -> TFrameIdx {
        self.duration
    }

    fn duration(&self) -> Option<TFrameIdx> {
        Some(self.duration)
    }

    fn get_by_frame_i(&mut self, frame_idx: TFrameIdx) -> Option<Vec<super::TSample>> {
        respond_storage!(self.storage_pile_id, frame_idx);

        if frame_idx != self.last_frame_idx + 1 {
            let seek_time = SeekTo::TimeStamp { ts: frame_idx as u64, track_id: self.track_id };
            self.reader.seek(SeekMode::Accurate, seek_time).ok()?;
        }

        let next_packet = self.reader.next_packet().ok()?;
        let decoded = self.decoder.decode(&next_packet).ok()?;

        let buf = SampleBuf::from_audio_buf_ref(next_packet.ts as TFrameIdx, &decoded);
        
        self.last_frame_idx = buf.start() + buf.frame_count() - 1;

        GLOBAL_SEGMENT_STORE.write().unwrap()
            .insert(self.storage_pile_id, buf.start(), buf.channels, buf.samples.into_boxed_slice());

        // let last_buf = mem::replace(&mut self.buf, buf);
        // self.cache = last_buf;

        self.get_by_frame_i(frame_idx)
    }
}