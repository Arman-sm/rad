use std::{sync::{Arc, Mutex, RwLock}, usize, vec};

use coarsetime::{Duration, Instant};

use crate::source::{BaseSource, Source};

pub type TWrappedCompositionState = Arc<RwLock<CompositionState>>;

static COMPOSITOR_ID_TO_ALLOCATE: Mutex<u16> = Mutex::new(0);
pub struct SrcCompositionData { pub frame_offset: isize, pub amplification: f32 }
pub struct CompositionSrc { pub src: Source, pub composition_data: SrcCompositionData }

pub fn convert_sample_rates(sample_rate_a: u32, rate_a: usize, sample_rate_b: u32) -> usize {
	rate_a * sample_rate_b as usize / sample_rate_a as usize
}

pub struct CompositionState {
	pub id: String,
	// TODO: Freeze elapsed time until pause is over
	pub pause_t: Option<Instant>,
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
	pub fn push_src_default(&mut self, src: Source) {
		self.sources.push(CompositionSrc {
			composition_data: SrcCompositionData { 
				amplification: 1.0,
				frame_offset: -(self.start_t.elapsed().as_f64() * src.sample_rate() as f64) as isize
			},
			src,
		});
	}

	pub fn push_src_offset(&mut self, src: Source, frame_offset: isize) {
		self.sources.push(CompositionSrc {
			composition_data: SrcCompositionData { 
				amplification: 1.0,
				frame_offset
			},
			src,
		});
	}

	pub fn get_time_millis(&self) -> u64 {
		self.pause_t.unwrap_or_else(|| Instant::now()).duration_since(self.start_t).as_millis()
	}

	pub fn set_time_millis(&mut self, millis: u64) {
		let now = Instant::now();

		if let Some(ref mut pause_t) = self.pause_t {
			let pause_gap = pause_t.duration_since(self.start_t);
			*pause_t = now - Duration::from_millis(millis.min(pause_gap.as_millis()));
		}
		
		self.start_t = now - Duration::from_millis(millis);
		self.config_change_idx += 1;
	}

	pub fn is_paused(&self) -> bool {
		self.pause_t.is_some()
	}

	pub fn set_paused(&mut self, state: bool) {
		if self.is_paused() == state { return; }
		
		if let Some(ref pause_t) = self.pause_t {
			self.set_time_millis(pause_t.duration_since(self.start_t).as_millis());
			self.pause_t = None;
		} else {
			self.pause_t = Some(Instant::now());
		}
	}
}

impl Default for CompositionState {
	fn default() -> Self {
		let mut id_handle = COMPOSITOR_ID_TO_ALLOCATE.lock().unwrap();
		let id = format!("DefaultCompositor{}", id_handle);

		*id_handle += 1;

		CompositionState {
			id,
			pause_t: None,
			channels: 2,
			sources: vec![],
			amplification: 1.5,
			config_change_idx: 0,
			start_t: Instant::now(),
		}
	}
}