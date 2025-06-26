use std::{sync::{atomic::AtomicU16, Arc, RwLock}, vec};

use coarsetime::Instant;

use crate::source::{BaseSource, Source, TFrameIdx};

pub type TWrappedCompositionState = Arc<RwLock<CompositionState>>;

static COMPOSITION_ID_TO_ALLOCATE: AtomicU16 = AtomicU16::new(0);
pub struct SrcCompositionData { pub frame_offset: i64, pub amplification: f32 }
pub struct CompositionSrc { pub src: Source, pub composition_data: SrcCompositionData }

pub fn convert_sample_rates(sample_rate_a: TFrameIdx, rate_a: TFrameIdx, sample_rate_b: TFrameIdx) -> TFrameIdx {
	rate_a * sample_rate_b / sample_rate_a
}

pub struct CompositionState {
	id: String,
	pause_t: Option<Instant>,
	channels: u8,
	pub sources: Vec<CompositionSrc>,
	amplification: f32,
	/// This field is used for checking whether `playback_offset_ms` has been changed and is used by the compositor to adapt accordingly.
	pub config_change_idx: u16,
	/// The anchor used to determine the elapsed time.
	start_t: Instant,
	/// Variable to adjust the playback time.
	/// Note: `config_change_idx` has to be incremented in order to properly notify the compositors of the change.
	playback_offset_ms: i64
}

impl CompositionState {
	pub fn new(id: String, amp: f32) -> Self {
		CompositionState {
			id,
			pause_t: None,
			channels: 2,
			sources: vec![],
			amplification: amp,
			config_change_idx: 0,
			start_t: Instant::now(),
			playback_offset_ms: 0
		}
	}

	// Adds a source with its start set to now and amplification of 1.0
	pub fn push_src_default(&mut self, src: Source) {
		self.sources.push(CompositionSrc {
			composition_data: SrcCompositionData { 
				amplification: 1.0,
				frame_offset: (self.get_time_sec() * src.sample_rate() as f64) as i64
			},
			src,
		});

	}

	pub fn push_src_offset(&mut self, src: Source, frame_offset: i64) {
		self.sources.push(CompositionSrc {
			composition_data: SrcCompositionData { 
				amplification: 1.0,
				frame_offset
			},
			src,
		});
	}

	pub fn get_time_millis(&self) -> u64 {
		let curr_now = self.pause_t.unwrap_or_else(|| Instant::now());
		let elapsed_time_ms = curr_now.duration_since(self.start_t).as_millis();
		
		elapsed_time_ms.saturating_add_signed(self.playback_offset_ms)
	}

	pub fn get_time_sec(&self) -> f64 {
		self.get_time_millis() as f64 / 1000.0
	}

	pub fn set_time_millis(&mut self, time_ms: u64) {
		let playback_time_ms = self.get_time_millis();
		self.playback_offset_ms += time_ms as i64 - playback_time_ms as i64;

		self.config_change_idx += 1;
	}

	pub fn is_pushed_pass_zero(&self) -> bool {
		self.playback_offset_ms < 0 && self.start_t.elapsed().as_millis() < self.playback_offset_ms.abs() as u64
	}

	pub fn is_paused(&self) -> bool {
		self.pause_t.is_some() || self.is_pushed_pass_zero()
	}

	pub fn set_paused_since(&mut self, time: Instant) {
		assert!(self.start_t < time);
		self.pause_t = Some(time);
	}

	pub fn set_paused(&mut self, state: bool) {
		if self.is_paused() == state { return; }
		
		if let Some(ref pause_t) = self.pause_t {
			let now = Instant::now();
			let time_passed_since_paused = now.duration_since(pause_t.clone());
			self.playback_offset_ms -= time_passed_since_paused.as_millis() as i64;
			
			self.pause_t = None;
		} else {
			self.pause_t = Some(Instant::now());
		}
	}

	pub fn start_time(&self) -> &Instant {
		&self.start_t
	}

	pub fn get_channels(&self) -> u8 {
		self.channels
	}

	pub fn get_id(&self) -> &String {
		&self.id
	}

	pub fn set_amplification(&mut self, amp: f32) {
		self.amplification = amp;
	}

	pub fn get_amplification(&self) -> f32 {
		self.amplification
	}
}

impl Default for CompositionState {
	fn default() -> Self {
		let id_handle = COMPOSITION_ID_TO_ALLOCATE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		let id = format!("cmp-{}", id_handle);

		CompositionState {
			id,
			pause_t: None,
			channels: 2,
			sources: vec![],
			amplification: 1.0,
			config_change_idx: 0,
			start_t: Instant::now(),
			playback_offset_ms: 0
		}
	}
}