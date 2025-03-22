// AdapterHandles are a way to manage different adapters ( outputs: e.g. host playback, udp adapter ) in a standardized way for the sake of simplicity and scalability.

use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[derive(Clone)]
pub struct AdapterHandle {
    id: String,
    is_closed: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
	kind: String
}

impl AdapterHandle {
	pub fn new(
		id: String, kind: String,
		status: Arc<Mutex<String>>,
		is_closed: Arc<AtomicBool>
	) -> Self {
		AdapterHandle {
			id,
			is_closed,
			kind,
			status
		}
	}

	pub fn close(&mut self) {
		use std::sync::atomic::Ordering;
		self.is_closed.store(true, Ordering::Relaxed);
	}

	pub fn id(&self) -> &str {
		&self.id
	}

	pub fn status(&self) -> String {
		self.status.lock().unwrap().clone()
	}

	pub fn kind(&self) -> &str {
		&self.kind
	}

	pub fn is_closed(&self) -> bool {
		use std::sync::atomic::Ordering;
		self.is_closed.load(Ordering::Relaxed)
	}
}