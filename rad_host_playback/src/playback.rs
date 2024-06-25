use std::sync::{atomic::AtomicBool, Arc, Mutex};

use rodio::{OutputStream, OutputStreamHandle};

use crate::writer::Writer;
use rad_compositor::{adapter::AdapterHandle, compositor::CompositionBufferNode};

#[allow(dead_code)]
pub struct HostPlayback {
	// Output stream
	out: OutputStream,
	out_handle: OutputStreamHandle,
}

// TODO: Add the possible errors
#[derive(Debug)]
pub enum PlaybackInitError {
	DeviceNotFound,
	// DeviceDisconnected,
}

pub fn init_host_playback_default(id: String, cmp_node: Arc<CompositionBufferNode<1024>>) -> (HostPlayback, AdapterHandle) {
	let status = Arc::new(Mutex::new("Playing".to_owned()));
	let is_closed = Arc::new(AtomicBool::new(false));

	let playback = HostPlayback::try_default(cmp_node, is_closed.clone()).expect("Failed to create new playback on host");

	(
		playback,
		AdapterHandle::new(id, "HostPlayback".to_owned(), status, is_closed)
	)
}

impl HostPlayback {
	// Pausing functionality now exists in the compositor
	//// pub fn play(&mut self)  { self.out_handle.().unwrap() }
	//// pub fn pause(&mut self) { self.out_handle..unwrap() }

	/// Initializes a new playback on the current host with the default output
	pub fn try_default(cmp_node: Arc<CompositionBufferNode<1024>>, is_closed: Arc<AtomicBool>) -> Result<Self, PlaybackInitError> {
		// TODO: Implement proper error casting
		let (out, out_handle) = rodio::OutputStream::try_default().unwrap();
		
		if let Err(err) = out_handle.play_raw(Writer::new(cmp_node.clone(), is_closed)) {
			eprintln!("{:?}", err)
		}
		
		Ok(HostPlayback {
			out,
			out_handle,
		})
	}
}