use std::{sync::{Arc, Mutex, RwLock}, usize, vec};

use coarsetime::{Duration, Instant};

use crate::source::Src;

pub type TWrappedCompositionState = Arc<RwLock<CompositionState>>;

static COMPOSITOR_ID_TO_ALLOCATE: Mutex<u16> = Mutex::new(0);
pub struct SrcCompositionData { pub frame_offset: isize, pub amplification: f32 }
pub struct CompositionSrc { pub src: Src, pub composition_data: SrcCompositionData }

pub enum FrameTime {
	// sample-rate, value
	Sample(u16, usize),
	Seconds(f32)
}

impl FrameTime {
	// Converts the time to the specified sample index
	pub fn to_sample_rate(&self, sample_rate: u16) -> usize {
		match self {
			FrameTime::Sample(self_sample_rate, value) => {
				if *self_sample_rate == sample_rate { return *value; }
				*value * sample_rate as usize / *self_sample_rate as usize
			},
			FrameTime::Seconds(secs) => {
				(secs * sample_rate as f32) as usize
			}
		}
	}

	// Creates a FrameTime based on a sample time
	pub fn from_sample(sample_rate: u16, value: usize) -> Self {
		Self::Sample(sample_rate, value)
	}

	// Creates a FrameTime based on seconds
	pub fn from_seconds(seconds: f32) -> Self {
		Self::Seconds(seconds)
	}
}

pub struct CompositionState {
	pub id: String,
	// TODO: Freeze elapsed time until pause is over
	pub is_paused: bool,
	pub channels: usize,
	pub sources: Vec<CompositionSrc>,
	pub amplification: f32,
	// This field is used for checking whether `start_t` has been changed and is used by the compositor to adapt accordingly.
	pub config_change_idx: u16,
	/// The anchor used to determine the elapsed time.
	/// By adjusting this field we can go back and fourth.
	/// Note: `config_change_idx` has to be incremented in order to properly notify the compositors of the change
	pub start_t: Instant,
}

impl CompositionState {
	// Adds a source with its start set to now and amplification of 1.0
	pub fn push_src_default(&mut self, src: Src) {
		self.sources.push(CompositionSrc {
			composition_data: SrcCompositionData { 
				amplification: 1.0,
				frame_offset: (self.start_t.elapsed().as_f64() * src.sample_rate as f64) as isize
			},
			src,
		});
	}

	pub fn set_time_millis(&mut self, millis: u64) {
		self.start_t = Instant::now() - Duration::from_millis(millis);
		self.config_change_idx += 1;
	}
}

impl Default for CompositionState {
	fn default() -> Self {
		let mut id_handle = COMPOSITOR_ID_TO_ALLOCATE.lock().unwrap();
		let id = format!("DefaultCompositor{}", id_handle);

		*id_handle += 1;

		CompositionState {
			id,
			is_paused: true,
			channels: 2,
			sources: vec![],
			amplification: 1.5,
			config_change_idx: 0,
			start_t: Instant::now(),
		}
	}
}