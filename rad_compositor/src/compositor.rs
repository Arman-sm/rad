use std::{mem, sync::{Arc, Condvar, Mutex}, thread::{self, sleep}, time::Duration};

use coarsetime::Instant;

use crate::{cmp_reg::{CompositorData, CompositorState}, composition::{convert_sample_rates, CompositionSrc, TWrappedCompositionState}, source::{BaseSource, Source, TSample}};

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

	/// Gives the last buffer generated in the list.
	pub fn head(&self) -> Arc<Self> {
		let mut head = self.next();
		
		loop {
			let next = head.next.lock().unwrap().clone();
			if let Some(node) = next {
				head = node;
			} else {
				break;
			}
		}

		head
	}

	pub fn live(&self, sample_rate: u32, channels: u16) -> Arc<Self> {
		let computed_buffers_ahead_of_time = (COMPUTE_AHEAD_SEC * sample_rate as f32) as usize / (BUF_SIZE / channels as usize);

		let mut res = self.next();

		let mut i = 1;
		let mut head = self.next();

		loop {
			let next = head.next.lock().unwrap().clone();
			if let Some(node) = next {
				head = node;
			} else {
				if i < computed_buffers_ahead_of_time {
					for _ in 0..(computed_buffers_ahead_of_time - i) {
						head.next();
					}
				}

				return res;
			}

			i += 1;
			if computed_buffers_ahead_of_time < i {
				res = res.next()
			}
		}
	}
}

fn approximate_frame_linear(src: &mut Source, sample_rate: u32, rate: usize, offset: isize) -> Option<Vec<f32>> {
	let conv = (rate * src.sample_rate() as usize) as f64 / sample_rate as f64 + offset as f64;
	let a = conv.floor() as usize;
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

pub fn compute_frame(sources: &mut [CompositionSrc], channels: u16, sample_rate: u32, frame_i: usize) -> Vec<TSample> {
	let mut res = vec![0f32; channels as usize];

	// Getting the output of each source
	for cmp_src in sources.iter_mut() {
		let src_res: Option<Vec<f32>>;	
		
		let conv_frame_i =
			convert_sample_rates(sample_rate, frame_i, cmp_src.src.sample_rate()) as isize
			- cmp_src.composition_data.frame_offset;

		if conv_frame_i < 0 { continue; }

		if cmp_src.src.sample_rate() == sample_rate {
			src_res = cmp_src.src.get_by_frame_i(conv_frame_i as usize);
		} else {
			src_res = approximate_frame_linear(&mut cmp_src.src, sample_rate, frame_i, cmp_src.composition_data.frame_offset);
		}
		
		if let Some(src_res) = src_res {
			for channel_i in 0..res.len() {
				res[channel_i] += src_res[channel_i % src_res.len()] * cmp_src.composition_data.amplification;
			}
		}
	}

	res
}

pub fn compute_frames<const BUF_SIZE: usize>(sources: &mut [CompositionSrc], channels: usize, sample_rate: u32, amplification: f32, offset: usize) -> [f32; BUF_SIZE] {
	let mut res = [0.0; BUF_SIZE];
	let n = BUF_SIZE / channels;

	for i in 0..n {
		let frame = compute_frame(sources, channels as u16, sample_rate, offset + i);
		for (ch_i, v) in frame.into_iter().enumerate() {
			res[i * channels + ch_i] = v * amplification;
		}
	}

	res
}

// pub fn amplify_frame

/// Initiates a new compositor to work on a separate thread and returns a pointer to the first buffer node which is an entry to the audio stream.
pub fn init_compositor_thread<const BUF_SIZE: usize>(sample_rate: u32, cmp_state: TWrappedCompositionState) -> (CompositorData<BUF_SIZE>, Arc<CompositionBufferNode<BUF_SIZE>>) {
	assert!(BUF_SIZE & 1 != 1);
	let first_node: Arc<CompositionBufferNode<BUF_SIZE>>;
	let channels;
	let cmp_id;
	let amp;
	{
		let mut cmp = cmp_state.write().unwrap();
		channels = cmp.channels;
		cmp_id = cmp.id.clone();
		amp = cmp.amplification;
		first_node = 
			CompositionBufferNode::new(compute_frames::<BUF_SIZE>(&mut cmp.sources, channels, sample_rate, amp, 0))
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
		.name(format!("cmp-{}", cmp_id))
		.spawn(move || {
			let start = Instant::now();

			let mut node = _first_node;
			let frames_in_buf = BUF_SIZE / channels;
			let mut frame_i = 0;
			let mut change_idx = 0;
			let mut secs_sent = 0.0;
			
			loop {
				// This condition ensures the compositor being killed in case of it not being used by anything
				// The compositor and the compositor registry each have a pointer to node so if the strong count of node is less than or equal to 2 it is not being used and so it gets killed.
				if Arc::strong_count(&node) < 3 {
					log::debug!("Killing compositor with state id of '{}' and sample rate of '{}'", _cmp_id, _sample_rate);
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
					secs_sent = cmp.start_t.elapsed().as_f64() as f32;
					frame_i = (secs_sent * sample_rate as f32) as usize;
					change_idx = cmp.config_change_idx;
				}

				node = node.push_next(
					compute_frames::<BUF_SIZE>(&mut cmp.sources, _channels, sample_rate, _amp, frame_i)
				);

				drop(cmp);

				secs_sent += 1.0 / (sample_rate as f32 / frames_in_buf as f32);
				frame_i += frames_in_buf;

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