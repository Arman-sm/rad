use std::{collections::HashSet, fs::File, io::Read, net::ToSocketAddrs, sync::{Arc, RwLock}};

use rad_compositor::{adapter::AdapterHandle, composition::{CompositionState, TWrappedCompositionState}, compositor::{CompositionBufferNode, init_compositor_thread}};
// use rad_host_playback::{init_host_playback_default, playback::HostPlayback};
use rad_net_stream::init_udp_adapter;
use serde::Deserialize;
use toml::Table;

#[derive(Deserialize)]
struct FileConfig {
	composition: Vec<Composition>,
	endpoints: Endpoints
}

#[derive(Deserialize)]
struct Composition {
	id: String,
	amp: f32,
	pause: bool,
}

#[derive(Deserialize)]
struct Endpoints {
	out: Vec<OutputEndpoint>,
}

#[derive(Deserialize)]
struct OutputEndpoint {
	id: String,
	adapter: String,
	ap: Table,
	cast: String,
	sample_rate: u16,
}

pub struct PState  {
	pub composition_states: Vec<TWrappedCompositionState>,
	pub adapters: Vec<AdapterHandle>
}

fn create_corresponding_composition_state(conf: &Composition) -> CompositionState {
	if conf.id.is_empty() {
		panic!("Composition ID can't be left empty.")
	}
	
	CompositionState {
		id: conf.id.clone(),
		amplification: conf.amp,
		is_paused: conf.pause,
		..Default::default()
	}
}

fn create_compositions(compositions: &Vec<Composition>) -> Vec<TWrappedCompositionState> {
	let mut ids = HashSet::new();
	let mut states = Vec::new();

	for cmp_conf in compositions.iter() {
		let cmp = create_corresponding_composition_state(cmp_conf);
		
		let has_id_existed = !ids.insert(cmp_conf.id.clone());
		if has_id_existed {
			panic!("Duplicate composition IDs were found, composition IDs must be unique.");
		}

		states.push(Arc::new(RwLock::new(cmp)));
	}

	states
}

struct WCmpBuf {
	buf: Arc<CompositionBufferNode<1024>>,
	id: String,
	sample_rate: u16,
}

fn create_corresponding_output_endpoint(buffers: &mut Vec<WCmpBuf>, compositions: &mut Vec<TWrappedCompositionState>, end_conf: &OutputEndpoint) -> AdapterHandle {
	
	let w_buf = buffers
		.iter()
		.find(|b| b.id == end_conf.cast && b.sample_rate == end_conf.sample_rate);

	let buf= match w_buf {
		Some(b) => b.buf.clone(),
		None => {
			let cmp = compositions
				.iter()
				.find(|c| c.read().unwrap().id == end_conf.cast)
				.expect(format!("Composition \"{}\" was not found. (set as \"cast\" in an endpoint config)", end_conf.id).as_str());

			let b = init_compositor_thread(44100, cmp.clone());

			buffers.push(WCmpBuf {
				id: end_conf.cast.clone(),
				sample_rate: end_conf.sample_rate,
				buf: b.clone()
			});

			b
		}
	};
	
	match end_conf.adapter.as_str() {
		"net-udp" => {
			let adapter_args = &end_conf.ap;
			
			let bind_addr =
				adapter_args.get("bind").expect("Filed \"ap:bind\" can't be left empty.")
				.as_str().expect("Field \"ap:bind\" has to be a string.")
				.to_socket_addrs().expect("Field \"ap:bind\" has to be a proper address.")
				.nth(0).expect("No addresses were found in \"ap:bind\".");

			let dest_addr =
				adapter_args.get("dest").expect("Filed \"ap:dest\" can't be left empty.")
				.as_str().expect("Field \"ap:dest\" has to be a string.")
				.to_socket_addrs().expect("Field \"ap:dest\" has to be a proper address.")
				.nth(0).expect("No addresses were found in \"ap:dest\".");

			init_udp_adapter(
				end_conf.id.clone(),
				bind_addr,
				dest_addr,
				buf
			)
		},
		"host" => {
			// TODO: Should keep the HostPlayback struct in order for the playback to work
			unimplemented!();
			// init_host_playback_default(end_conf.id, buf)
		},
		_ => {
			panic!("")
		}
	}
}

// For now only output endpoints will be supported but support endpoints for receiving audio from devices like microphones on external devices may be implemented in the feature but isn't a planned feature yet.
fn create_endpoints(compositions: &mut Vec<TWrappedCompositionState>, endpoints_config: &Endpoints) -> Vec<AdapterHandle> {
	let mut adapters = Vec::with_capacity(endpoints_config.out.len());
	let mut ids = HashSet::new();
	let mut buffers = Vec::new();

	for end_conf in endpoints_config.out.iter() {
		if end_conf.id.is_empty() {
			panic!("Endpoint ID can't be empty.")
		}
		
		let has_id_existed = !ids.insert(end_conf.id.clone());
		if has_id_existed {
			panic!("Duplicate endpoint IDs were found, endpoint IDs must be unique.");
		}

		adapters.push(
			create_corresponding_output_endpoint(&mut buffers, compositions, &end_conf)
		);		
	}

	adapters
}

/// This function with panic in case of encountering any errors while trying to read and set the program up according to it.
/// As in any case the program is not intended to be ran in case of a faulty configuration file.
pub fn init_with_file_config(path: &str) -> PState {
	log::debug!("Reading the configuration file at \"{path}\".");
	
	let mut raw_config = String::new();
	
	if let Ok(mut file) = File::open(path) {
		let read_res = file.read_to_string(&mut raw_config);
		if let Err(e) = read_res {
			panic!("Failed to read the configuration file at \"{path}\" due to io error \"{e}\".");
		}
	}

	let config: FileConfig =
		match toml::from_str(&raw_config) {
			Ok(v) => v,
			Err(_) => panic!("Failed to parse \"{path}\".")
		};
	
	let mut cmp_states = create_compositions(&config.composition);

	let out_adapters = create_endpoints(&mut cmp_states, &config.endpoints);

	PState {
		composition_states: cmp_states,
		adapters: out_adapters,
	}
}