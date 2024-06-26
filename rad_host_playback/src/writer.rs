use std::sync::{atomic::AtomicBool, Arc};

use rad_compositor::{compositor::CompositionBufferNode, source::TSample};

pub struct Writer {
	channels: u16,
    sample_rate: u32,
	is_closed: Arc<AtomicBool>,
	buf_idx: usize,
	cmp_node: Arc<CompositionBufferNode<1024>>
}

impl Writer {
	pub fn new(sample_rate: u32, channels: u16, cmp_node: Arc<CompositionBufferNode<1024>>, is_closed: Arc<AtomicBool>) -> Self {
		Writer {
			channels,
			sample_rate,
			is_closed,
			buf_idx: 0,
			cmp_node,
		}
	}
}

impl rodio::Source for Writer {
	fn channels(&self) -> u16 { self.channels }
	fn current_frame_len(&self) -> Option<usize> { None }
	fn sample_rate(&self) -> u32 { self.sample_rate }
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