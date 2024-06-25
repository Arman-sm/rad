use std::{sync::{Arc, Condvar, Mutex}, thread::{self, sleep}, time::Duration};

use coarsetime::Instant;

use crate::{composition::{CompositionState, FrameTime, TWrappedCompositionState}, source::TSample};

const COMPUTE_AHEAD_SEC: f32 = 1.0;

/// The `BUF_SIZE` argument means how many f32s will fit inside of this buffer
pub struct CompositionBufferNode<const BUF_SIZE: usize>  {
	cnxt: Condvar,
	next: Mutex<Option<Arc<CompositionBufferNode<BUF_SIZE>>>>,
	buffer: [f32; BUF_SIZE]
}

impl<const BUF_SIZE: usize> CompositionBufferNode<BUF_SIZE> {
	pub fn new(buf: [f32; BUF_SIZE]) -> Arc<Self> {
		let res = Arc::new(
			CompositionBufferNode {
				buffer: buf,
				cnxt: Condvar::new(),
				next: Mutex::new(None),
			}
		);

		res
	}

	pub fn push_next(&self, buf: [f32; BUF_SIZE]) -> Arc<Self> {
		let mut next_lock = self.next.lock().unwrap();
		if next_lock.is_some() { panic!("Tried to push the next buf while it already existed.") }
		
		let next_node = Self::new(buf);

		*next_lock = Some(next_node.clone());
		self.cnxt.notify_all();

		next_node
	}

	pub fn next(&self) -> Arc<Self> {
		let mut next = self.next.lock().unwrap();
		while next.is_none() {
			next = self.cnxt.wait(next).unwrap();
		}

		next.as_ref().unwrap().clone()
	}
	
	pub fn buf(&self) -> &[f32; BUF_SIZE] {
		&self.buffer
	}
}

pub fn compute_frame(cmp: &mut CompositionState, frame_time: FrameTime) -> Vec<TSample> {
	let mut res = vec![0f32; cmp.channels];

	// Getting the output of each source
	for src_i in 0..cmp.sources.len() {
		// TODO: This method of sample rate conversion (Nearest-Neighbor Interpolation) wildly decreases the audio quality, and so a proper sample-rate conversion algorithm has to be implemented.
		// Temporary suggestion: Linear Interpolation -> Simple and fast but still decreased the quality by a lot.
		let frame_i = frame_time.to_sample_rate(cmp.sources[src_i].src.sample_rate);
		let src_frame_i = frame_i as isize - cmp.sources[src_i].composition_data.frame_offset;
		
		if src_frame_i < 0 { continue; }

		if let Some(v) = cmp.sources[src_i].src.get_by_frame_i(src_frame_i as usize) {
			for v_i in 0..res.len().min(v.len()) {
				res[v_i] += v[v_i] * cmp.sources[src_i].composition_data.amplification;
			}
		}
	}

	for v in res.iter_mut() {
		*v *= cmp.amplification;
	}

	res
}

pub fn compute_frames<const BUF_SIZE: usize>(sample_rate: u16, cmp: &mut CompositionState, offset: usize) -> [f32; BUF_SIZE] {
	let mut res = [0.0; BUF_SIZE];
	let channels = cmp.channels;
	let n = BUF_SIZE / channels;

	for i in 0..n {
		let frame_time = FrameTime::from_sample(sample_rate, offset + i);
		let frame = compute_frame(cmp, frame_time);
		for (ch_i, v) in frame.into_iter().enumerate() {
			res[i * channels + ch_i] = v;
		}
	}

	res
}

pub fn init_compositor_thread<const BUF_SIZE: usize>(sample_rate: u16, cmp_state: TWrappedCompositionState) -> Arc<CompositionBufferNode<BUF_SIZE>>
{	
	assert!(BUF_SIZE & 1 != 1);
	let first_node: Arc<CompositionBufferNode<BUF_SIZE>>;
	let channels;
	let cmp_id;
	{
		let mut cmp = cmp_state.write().unwrap();
		channels = cmp.channels;
		cmp_id = cmp.id.clone();
		first_node = 
			CompositionBufferNode::new(compute_frames::<BUF_SIZE>(sample_rate, &mut cmp, 0))
	}
	let _first_node = first_node.clone();

	let _thread_handle = thread::Builder::new()
		.name(format!("cmp-{}", cmp_id))
		.spawn(move || {
			let start = Instant::now();
			let mut buf_computed = 0;

			let mut node = first_node;
			let frames_in_buf = BUF_SIZE / channels;
			let mut i = 0;
			let mut change_idx = 0;

			loop {
				let secs_sent = (buf_computed * frames_in_buf) as f32 / 44100.0 as f32;
				if start.elapsed().as_f64() as f32 - secs_sent < COMPUTE_AHEAD_SEC {
					sleep(Duration::from_secs_f32(0.05));
					continue;
				}

				i += 1;
				
				let mut cmp = cmp_state.write().unwrap();
				if cmp.is_paused {
					drop(cmp);
					sleep(Duration::from_secs_f32(0.05));
					continue;
				}

				if change_idx < cmp.config_change_idx {
					i = (cmp.start_t.elapsed().as_f64() as f32 * sample_rate as f32) as usize / frames_in_buf;
					change_idx = cmp.config_change_idx;
				}

				buf_computed += 1;
				node = node.push_next(
					compute_frames::<BUF_SIZE>(sample_rate, &mut cmp, i * frames_in_buf)
				);
			}
		}).unwrap();

	// TODO: Return the JoinHandle
	_first_node
}