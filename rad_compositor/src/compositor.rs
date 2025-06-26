use std::{sync::{Arc, Condvar, Mutex}, thread::{self, sleep}, time::Duration};

use coarsetime::Instant;

use crate::{cmp_reg::{CompositorData, CompositorState}, composition::{convert_sample_rates, CompositionSrc, TWrappedCompositionState}, source::{BaseSource, Source, TFrameIdx, TSample}};

const COMPUTE_AHEAD_SEC: f32 = 0.3;

/// A linked list like structure for streaming audio that supports multithreading. <br/>
/// 
/// Note: The `BUF_SIZE` argument means how many f32s will fit inside of this buffer
pub struct CompositionBufferNode<const BUF_SIZE: usize>  {
	cnxt: Condvar,
	next: Mutex<Option<Arc<CompositionBufferNode<BUF_SIZE>>>>,
	buf: [f32; BUF_SIZE]
}

impl<const BUF_SIZE: usize> CompositionBufferNode<BUF_SIZE> {
	pub fn new(buf: [f32; BUF_SIZE]) -> Arc<Self> {
		let res = Arc::new(
			CompositionBufferNode {
				buf,
				cnxt: Condvar::new(),
				next: Mutex::new(None),
			}
		);

		res
	}

	/// Sets the next buffers and causes the other threads waiting for the next buffer to resume.
	/// Caution: This method panics if the next buffer is already set.
	pub fn push_next(&self, buf: [f32; BUF_SIZE]) -> Arc<Self> {
		let mut next_lock = self.next.lock().unwrap();
		if next_lock.is_some() { panic!("Tried to push the next buf while it already existed.") }
		
		let next_node = Self::new(buf);

		*next_lock = Some(next_node.clone());
		self.cnxt.notify_all();

		next_node
	}

	/// Gives the next buffer. <br/>
	/// Caution: This method **will wait** for the compositor thread to generate the next buffer if it wasn't generated.
	pub fn next(&self) -> Arc<Self> {
		let mut next = self.next.lock().unwrap();
		while next.is_none() {
			next = self.cnxt.wait(next).unwrap();
		}

		next.as_ref().unwrap().clone()
	}
	
	/// Gives a reference to the buffer data.
	pub fn buf(&self) -> &[f32; BUF_SIZE] {
		&self.buf
	}

	/// Sets the current node to the last node generated.
	pub fn set_to_head(node: &mut Arc<CompositionBufferNode<BUF_SIZE>>) {
		loop {
			let next = node.next.lock().unwrap().clone();
			if let Some(next_node) = next {
				*node = next_node;
			} else {
				break;
			}
		}
	}

	pub fn set_to_live(node: &mut Arc<CompositionBufferNode<BUF_SIZE>>, sample_rate: TFrameIdx, channels: u8) {
		let computed_buffers_ahead_of_time = (COMPUTE_AHEAD_SEC * sample_rate as f32) as usize / (BUF_SIZE / channels as usize);

		Self::set_to_head(node);
		let mut head = node.clone();

		for i in 0..computed_buffers_ahead_of_time {
			head = head.next();
		}
	}
}

/// Makes the sample-rate conversion a bit smoother.
/// 
/// Finds the nearest two frames in the source sample-rate and calculates the weighted average of them based on their closeness to the target.
pub fn approximate_frame_linear(src: &mut Source, into_sample_rate: TFrameIdx, rate: TFrameIdx, offset: i64) -> Option<Vec<TSample>> {
	let conv = (rate * src.sample_rate()) as f64 / into_sample_rate as f64 - offset as f64;
	assert!(0.0 <= conv);

	let a = conv.floor() as TFrameIdx;
	let diff = conv - conv.floor();

	let res_a = src.get_by_frame_i(a)?;
	let res_b = src.get_by_frame_i(a + 1)?;

	let len = res_a.len().min(res_b.len());
	let mut res = Vec::new();
	res.reserve_exact(len);

	for i in 0..len {
		let sp_diff = res_b[i] - res_a[i];
		res.push(res_a[i] + (sp_diff as f64 * diff) as f32);
	}

	Some(res)
}

fn fetch_frame(cmp_src: &mut CompositionSrc, target_sample_rate: TFrameIdx, frame_idx: TFrameIdx) -> Option<Vec<TSample>> {
	let conv_frame_i =
			convert_sample_rates(target_sample_rate, frame_idx, cmp_src.src.sample_rate()) as i64
			- cmp_src.composition_data.frame_offset;

	if conv_frame_i < 0 { return None; }

	if cmp_src.src.sample_rate() == target_sample_rate {
		cmp_src.src.get_by_frame_i(conv_frame_i as TFrameIdx)
	} else {
		approximate_frame_linear(&mut cmp_src.src, target_sample_rate, frame_idx, cmp_src.composition_data.frame_offset)
	}
}

pub fn compute_eventual_frame(sources: &mut [CompositionSrc], channels: u8, sample_rate: TFrameIdx, frame_idx: TFrameIdx) -> Vec<TSample> {
	let mut res = vec![0f32; channels as usize];

	// Getting the output of each source
	for cmp_src in sources.iter_mut() {
		let conv_frame_i =
			convert_sample_rates(sample_rate, frame_idx, cmp_src.src.sample_rate()) as i64
			- cmp_src.composition_data.frame_offset;

		if conv_frame_i < 0 { continue; }

		let val = fetch_frame(cmp_src, sample_rate, frame_idx);

		if let Some(src_res) = val {
			for channel_i in 0..res.len() {
				res[channel_i] += src_res[channel_i % src_res.len()] * cmp_src.composition_data.amplification;
			}
		}
	}

	res
}

pub fn compute_frames<const BUF_SIZE: usize>(sources: &mut [CompositionSrc], channels: u8, sample_rate: TFrameIdx, amplification: f32, offset: TFrameIdx) -> [f32; BUF_SIZE] {
	let mut res = [0.0; BUF_SIZE];
	let n = BUF_SIZE / channels as usize;

	for i in 0..n {
		let frame = compute_eventual_frame(sources, channels, sample_rate, offset + i as TFrameIdx);
		for (ch_i, v) in frame.into_iter().enumerate() {
			res[i * channels as usize + ch_i] = v * amplification;
		}
	}

	res
}

/// Initiates a new compositor to work on a separate thread and returns a pointer to the first buffer node which is an entry to the audio stream.
pub fn init_compositor_thread<const BUF_SIZE: usize>(sample_rate: TFrameIdx, cmp_state: TWrappedCompositionState) -> (CompositorData<BUF_SIZE>, Arc<CompositionBufferNode<BUF_SIZE>>) {
	assert!(BUF_SIZE & 1 != 1);
	let first_node: Arc<CompositionBufferNode<BUF_SIZE>>;
	let channels;
	let cmp_id;
	let amp;
	{
		let mut cmp = cmp_state.write().unwrap();
		channels = cmp.get_channels();
		cmp_id = cmp.get_id().clone();
		amp = cmp.get_amplification();
		first_node = 
			CompositionBufferNode::new(compute_frames::<BUF_SIZE>(&mut cmp.sources, channels.into(), sample_rate, amp, 0))
	}
	
	// Saving the current thread id does not mean anything. Its just a valid ThreadId to put instead of mem::uninitialized until changing it after creating the compositor thread.
	let state = Arc::new(Mutex::new(CompositorState::Active(thread::current().id(), first_node.clone())));
	
	let _state = state.clone();
	let _sample_rate = sample_rate.clone();
	let _channels = channels;
	let _cmp_id = cmp_id.clone();
	let _amp = amp;
	let _first_node = first_node.clone();
	
	let thread_handle = thread::Builder::new()
		.name(format!("cmp-{}/{}", cmp_id, sample_rate))
		.spawn(move || {
			let mut start = Instant::now();

			let mut node = _first_node;
			let frames_in_buf: TFrameIdx = (BUF_SIZE / channels as usize) as TFrameIdx;
			let mut frame_idx: TFrameIdx = 0;
			let mut change_idx = 0;
			let mut secs_sent = 0.0;
			
			loop {
				// This condition ensures the compositor being killed in case of it not being used by anything
				// The compositor and the compositor registry each have a pointer to node so if the strong count of node is less than or equal to 2 it is not being used and so it gets killed.
				if Arc::strong_count(&node) < 3 {
					log::debug!("Killing compositor with state id of '{}' and sample-rate of '{}'", _cmp_id, _sample_rate);
					*state.lock().unwrap() = CompositorState::Killed;
					return;
				}
				
				// This section is dedicated to preventing the compositor from computing too much audio as adjustments can be made live.
				if COMPUTE_AHEAD_SEC < secs_sent - start.elapsed().as_f64() as f32 {
					sleep(Duration::from_secs_f32(0.05));
					continue;
				}
				
				let mut cmp = cmp_state.write().unwrap();
				if cmp.is_paused() {
					drop(cmp);
					sleep(Duration::from_secs_f32(0.05));
					continue;
				}

				if change_idx < cmp.config_change_idx {
					start = Instant::now();
					secs_sent = 0.0;
					frame_idx = (cmp.get_time_sec() as f32 * sample_rate as f32) as TFrameIdx;
					change_idx = cmp.config_change_idx;
				}

				node = node.push_next(
					compute_frames::<BUF_SIZE>(&mut cmp.sources, _channels, sample_rate, _amp, frame_idx)
				);

				drop(cmp);

				secs_sent += 1.0 / (sample_rate as f32 / frames_in_buf as f32);
				frame_idx += frames_in_buf;

				if let CompositorState::Active(_, ref mut state_buff_p) = *state.lock().unwrap() {
					*state_buff_p = node.clone();
				}
			}
		}).unwrap();

	if let CompositorState::Active(ref mut thread_id, _) = *_state.lock().unwrap() {
		*thread_id = thread_handle.thread().id();
	}

	(CompositorData::new(cmp_id, sample_rate, _state), first_node)
}