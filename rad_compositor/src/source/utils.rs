use std::{fs::read_dir, ops::Range, path::{Path, PathBuf}};

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal};

use crate::source::file::FileSource;

use super::{iter::{IterSrcFunc, TIterSrcFuncReturn}, queue::QueueSrc, TSample};

#[derive(Clone)]
pub struct SampleBuf {
	pub samples: Vec<TSample>,
	start_i: usize,
}

impl SampleBuf {
	pub fn new(start_i: usize, samples: Vec<TSample>) -> Self {
		SampleBuf {
			start_i,
			samples
		}
	}

	pub fn start(&self) -> usize {
		self.start_i
	}

	pub fn end(&self) -> usize {
		self.start_i + self.samples.len()
	}

	pub fn len(&self) -> usize {
		self.samples.len()
	}

	pub fn range(&self) -> Range<usize> {
		self.start_i..self.end()
	}

	pub fn from_audio_buf_ref(start_i: usize, audio_buf_ref: &AudioBufferRef) -> SampleBuf {
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

	pub fn from_audio_buf(start_i: usize, samp_buf: &AudioBuffer<f32>) -> Self {
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
			start_i
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

pub fn queue_from_directory(path: &Path, sample_rate: u32, depth: u8) -> Option<QueueSrc> {
	let mut queue = QueueSrc::new(sample_rate);	
	let file_paths = find_files(path, depth)?;
	for file_path in file_paths {
		log::debug!("Reading '{}'", file_path.display());

		let src = FileSource::new(file_path)?;

		queue.push(src.into());
	}
		
	Some(queue)
}

pub fn delay_iter(mut func: Box<IterSrcFunc>, mut first_call_return: TIterSrcFuncReturn) -> Box<IterSrcFunc> {
	let mut is_first_call = true;
	
	Box::new(move || {
		if is_first_call { 
			is_first_call = false;
			return std::mem::take(&mut first_call_return);
		}

		func()
	})
}