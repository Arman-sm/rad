use std::{ops::Range, sync::Mutex};

pub mod utils;

pub type TSample = f32;
pub type TSrcFuncReturn = Option<Vec<TSample>>;

pub struct IterData { pub has_ended: bool }

pub type IterSrcFunc = dyn FnMut() -> TSrcFuncReturn + Send + Sync;

pub enum SrcFunc {
	// TODO: Add dynamic src support
	Iter(Box<IterSrcFunc>, IterData),
}

/// This function's job is to prevent calling SrcFunc::Iter(func, data) after it has ended
fn call_src_iter(func: &mut Box<IterSrcFunc>, iter_data: &mut IterData) -> TSrcFuncReturn {
	if iter_data.has_ended { return None }

	let payload = func();

	if let None = payload {
		iter_data.has_ended = true;
	}

	payload
}

#[derive(Clone)]
pub struct SampleBuff {
	samples: Vec<TSample>,
	start_sample_i: usize,
}

impl SampleBuff {
	pub fn new(start: usize, samples: Vec<TSample>) -> Self {
		SampleBuff {
			start_sample_i: start,
			samples
		}
	}

	pub fn end(&self) -> usize {
		self.start_sample_i + self.samples.len()
	}

	pub fn len(&self) -> usize {
		self.samples.len()
	}

	pub fn range(&self) -> Range<usize> {
		self.start_sample_i..self.end()
	}
}

static SRC_ALLOCATED_IDX: Mutex<u16> = Mutex::new(0);

pub struct Src {
	pub sample_rate: u16,
	pub channels: usize,
	pub index: u16,
	// ! The buffers in cache have to be sorted, as binary search will be used when searching them.
	cache: Vec<SampleBuff>,
	buf: SampleBuff,
	func: SrcFunc,

}

impl Src {
	pub fn new(func: SrcFunc, sample_rate: u16, channels: usize) -> Self {
		let mut last_index = SRC_ALLOCATED_IDX.lock().unwrap();

		*last_index += 1;

		Src {
			sample_rate,
			channels,
			index: *last_index,
			cache: Vec::new(),
			buf: SampleBuff::new(0, Vec::new()),
			func
		}
	}

	pub fn get_by_frame_i(&mut self, frame_i: usize) -> TSrcFuncReturn {		
		if !self.buf.range().contains(&(frame_i * self.channels)) {
			// In this case, `iter_func` has ended so there won't be any data to return
			if !self.load_buf(frame_i) {
				// eprintln!("Failed to load the next buffer");
				return None
			}
		}

		let buff_start_idx = frame_i * self.channels - self.buf.start_sample_i;
		Some(self.buf.samples[buff_start_idx..buff_start_idx + self.channels].to_vec())
	}

	/// returns wether the operation was successful or not.
	/// If it wasn't then the buffer **must not be used to retrieve a frame outside of it**.
	// TODO: Add proper error casting
	fn load_buf(&mut self, frame_i: usize) -> bool {
		// if frame_i * self.channels < self.buf.start_sample_i {
		if !self.cache.is_empty() && frame_i * self.channels < self.cache.last().unwrap().end() {
			self.buf = self.cache[self.search_cache_for_buf(frame_i * self.channels)].clone();
			return true
		}

		match self.func {
			SrcFunc::Iter(ref mut func, ref mut iter_data) => {
				if iter_data.has_ended { return false }

				while self.buf.end() < (frame_i + 1) * self.channels {
					let _data = call_src_iter(func, iter_data);

					if let Some(data) = _data {
						self.buf = SampleBuff::new(self.buf.end(), data);
						self.cache.push(self.buf.clone());
					} else {
						iter_data.has_ended = true;
						return false
					}
				}

				assert!(self.buf.samples.len() % self.channels == 0);
				assert!(self.buf.len() != 0);
				return true
			}
		}
	}

	/// This function will panic if `self.cache` doesn't contain `frame_i`
	/// Parameter `i` is the index of the value of an specific channel that you want. (each SampleBuffer contains i values (f32s))
	fn search_cache_for_buf(&self, i: usize) -> usize { // Option<usize> {
		let mut start = 0;
		let mut end = self.cache.len() - 1;

		let res = loop {
			if start == end { break start; }

			let idx = (start + end) / 2;
			let selected = &self.cache[idx];

			if i < selected.start_sample_i { end = idx - 1 }
			else if i >= selected.end() { start = idx + 1 }
			else { break idx; }
		};

		assert!(self.cache[res].range().contains(&i));

		res
	}
}

pub fn iter_func(func: Box<IterSrcFunc>) -> SrcFunc {
	SrcFunc::Iter(func, IterData { has_ended: false })
}