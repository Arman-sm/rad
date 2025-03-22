use std::{fs::read_dir, ops::Range, path::{Path, PathBuf}};

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};

use crate::source::{formatted::FormattedStreamSource, queue::QueueSrc, TFrameIdx, TSample};

#[derive(Clone)]
pub struct SampleBuf {
	pub samples: Vec<TSample>,
	pub channels: u8,
	start_idx: TFrameIdx,
}

impl SampleBuf {
	pub fn new(start_idx: TFrameIdx, samples: Vec<TSample>, channels: u8) -> Self {
		SampleBuf {
			start_idx,
			channels,
			samples
		}
	}

	pub fn start(&self) -> TFrameIdx {
		self.start_idx
	}

	#[deprecated]
	pub fn end(&self) -> TFrameIdx {
		self.start_idx + self.samples.len() as TFrameIdx
	}

	pub fn frame_count(&self) -> TFrameIdx {
		self.sample_count() / self.channels as TFrameIdx
	}

	pub fn sample_count(&self) -> TFrameIdx {
		self.samples.len() as TFrameIdx
	}

	pub fn real_frame_end(&self) -> TFrameIdx {
		self.start_idx + self.frame_count()
	}

	pub fn real_frame_range(&self) -> Range<TFrameIdx> {
		self.start_idx..(self.start_idx + self.sample_count() / self.channels as TFrameIdx)
	}

	#[deprecated]
	#[allow(deprecated)]
	pub fn range(&self) -> Range<TFrameIdx> {
		self.start_idx..self.end()
	}

	pub fn from_audio_buf_ref(start_i: TFrameIdx, audio_buf_ref: &AudioBufferRef) -> SampleBuf {
		let f32_buf;
		
		match audio_buf_ref {
            AudioBufferRef::U8(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::U16(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::U24(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::U32(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::S8(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::S16(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::S24(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::S32(buf) => { f32_buf = buf.make_equivalent::<f32>() },
            AudioBufferRef::F32(buf) => { return Self::from_audio_buf(start_i, &buf); },
            AudioBufferRef::F64(buf) => { f32_buf = buf.make_equivalent::<f32>() },
        };

		Self::from_audio_buf(start_i, &f32_buf)
	}

	pub fn from_audio_buf(start_i: TFrameIdx, samp_buf: &AudioBuffer<f32>) -> Self {
		let channels = samp_buf.spec().channels.count();
		let frames = samp_buf.frames();
		let samples = frames * channels;

		let mut buf = Vec::with_capacity(samples);

		let channel_bufs = (0..channels).into_iter().map(|ch_i| samp_buf.chan(ch_i)).collect::<Vec<_>>();

		for frame_i in 0..frames {
			for ch_i in 0..channels {
				buf.push(channel_bufs[ch_i][frame_i]);
			}
		}

		SampleBuf {
			samples: buf,
			channels: channels as u8,
			start_idx: start_i
		}
	}
}

fn find_files(path: &Path, depth: u8) -> Option<Vec<PathBuf>> {
	let mut files = Vec::new();
	
	let dir_iter = read_dir(path).ok()?;
	for item in dir_iter {
		let item = item.ok()?;
		if depth != 0 && item.file_type().ok()?.is_dir() {
			let mut sub_files = find_files(item.path().as_path(), depth - 1)?;
			files.append(&mut sub_files);
		} else {
			files.push(item.path());
		}
	}

	Some(files)
}

pub fn queue_from_directory(path: &Path, sample_rate: TFrameIdx, depth: u8) -> Option<QueueSrc> {
	let mut queue = QueueSrc::new(sample_rate);	
	let file_paths = find_files(path, depth)?;
	for file_path in file_paths {
		log::debug!("Reading '{}'", file_path.display());

		let src = FormattedStreamSource::open_path(file_path)?;

		queue.push(src.into());
	}
		
	Some(queue)
}