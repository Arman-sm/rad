use std::sync::{atomic::AtomicBool, Arc};

use rad_compositor::{compositor::CompositionBufferNode, source::TSample};

pub struct Writer {
	is_closed: Arc<AtomicBool>,
	buf_idx: usize,
	cmp_node: Arc<CompositionBufferNode<1024>>
}

impl Writer {
	pub fn new(cmp_node: Arc<CompositionBufferNode<1024>>, is_closed: Arc<AtomicBool>) -> Self {
		Writer {
			is_closed,
			buf_idx: 0,
			cmp_node,
		}
	}
}

// TODO: Make these configurable
impl rodio::Source for Writer {
	fn channels(&self) -> u16 { 2 }
	fn current_frame_len(&self) -> Option<usize> { None }
	fn sample_rate(&self) -> u32 { 44100 }
	fn total_duration(&self) -> Option<std::time::Duration> { None }
}

impl Iterator for Writer {
	type Item = TSample;

	fn next(&mut self) -> Option<TSample> {
		if self.cmp_node.buf().len() == self.buf_idx {
			self.buf_idx = 0;
			self.cmp_node = self.cmp_node.next();
		}

		use std::sync::atomic::Ordering;
		if self.is_closed.load(Ordering::Relaxed) { return None; }

		let res = self.cmp_node.buf()[self.buf_idx];
		self.buf_idx += 1;

		Some(res)
	}
}