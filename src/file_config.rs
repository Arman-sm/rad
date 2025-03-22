use std::{collections::HashSet, fs::File, io::Read, net::{SocketAddr, ToSocketAddrs}, sync::{Arc, Mutex, RwLock}};

use rad_compositor::{adapter::AdapterHandle, cmp_reg::CompositionRegistry, composition::CompositionState, source::TFrameIdx};
use rad_net_stream::{init_simple_http_adapter, init_udp_adapter};
use serde::Deserialize;
use toml::Table;

const DEFAULT_REMOTE_ADDR: &str = "127.0.0.1:4600";

#[derive(Deserialize)]
struct FileConfig {
	// TODO: Change this field's name.
	api_addr: Option<String>,
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
	sample_rate: TFrameIdx,
	channels: u8
}

pub struct PState  {
	pub remote_addr: SocketAddr,
	pub cmp_reg: Arc<Mutex<CompositionRegistry<1024>>>,
	pub adapters: Vec<AdapterHandle>
}

fn create_corresponding_composition_state(conf: &Composition) -> CompositionState {
	if conf.id.is_empty() {
		panic!("Composition ID can't be left empty.")
	}

	let mut res = CompositionState::new(conf.id.clone(), conf.amp);

	if conf.pause {
		res.set_paused_since(res.start_time().clone());
	}

	res
}

fn create_composition_registry<const BUF_SIZE: usize>(compositions: &Vec<Composition>) -> CompositionRegistry<BUF_SIZE> {
	let mut ids = HashSet::new();
	let mut reg = CompositionRegistry::new();

	for cmp_conf in compositions.iter() {
		let cmp = create_corresponding_composition_state(cmp_conf);
		
		let has_id_existed = !ids.insert(cmp_conf.id.clone());
		if has_id_existed {
			panic!("Duplicate composition IDs were found, composition IDs must be unique.");
		}

		reg.push_composition(Arc::new(RwLock::new(cmp)));
	}

	reg
}

fn create_corresponding_output_endpoint(cmp_reg: Arc<Mutex<CompositionRegistry<1024>>>, end_conf: &OutputEndpoint) -> AdapterHandle {	
	match end_conf.adapter.as_str() {
		"net-udp" => {
			let adapter_args = &end_conf.ap;
			
			let bind_addr =
				adapter_args.get("bind").expect("Filed 'ap:bind' can't be left empty.")
				.as_str().expect("Field 'ap:bind' has to be a string.")
				.to_socket_addrs().expect("Field 'ap:bind' has to be a proper address.")
				.nth(0).expect("No addresses were found in 'ap:bind'.");

			let dest_addr =
				adapter_args.get("dest").expect("Filed 'ap:dest' can't be left empty.")
				.as_str().expect("Field 'ap:dest' has to be a string.")
				.to_socket_addrs().expect("Field 'ap:dest' has to be a proper address.")
				.nth(0).expect("No addresses were found in 'ap:dest'.");

			init_udp_adapter(
				end_conf.id.clone(),
				bind_addr,
				dest_addr,
				cmp_reg.lock().unwrap().get_active_buf(&end_conf.cast, end_conf.sample_rate).unwrap()
			)
		},
		"net-simple-http" => {
			let adapter_args = &end_conf.ap;
			
			let bind_addr =
				adapter_args.get("bind").expect("Filed 'ap:bind' can't be left empty.")
				.as_str().expect("Field 'ap:bind' has to be a string.")
				.to_socket_addrs().expect("Field 'ap:bind' has to be a proper address.")
				.nth(0).expect("No addresses were found in 'ap:bind'.");

			init_simple_http_adapter(
				end_conf.id.clone(),
				end_conf.sample_rate,
				end_conf.channels,
				bind_addr,
				end_conf.cast.clone(),
				cmp_reg.clone()
			)
		},
		"host" => {
			// TODO: Should keep the HostPlayback struct in order for the playback to work
			unimplemented!();
			// init_host_playback_default(end_conf.id, buf)
		},
		other_type => {
			panic!("Invalid adapter type '{}' was chosen in the configuration file.", other_type)
		}
	}
}

// For now only output endpoints will be supported but support endpoints for receiving audio from devices like microphones on external devices may be implemented in the feature but isn't a planned feature yet.
fn create_endpoints(cmp_reg: Arc<Mutex<CompositionRegistry<1024>>>, endpoints_config: &Endpoints) -> Vec<AdapterHandle> {
	let mut adapters = Vec::with_capacity(endpoints_config.out.len());
	let mut ids = HashSet::new();

	for end_conf in endpoints_config.out.iter() {
		if end_conf.id.is_empty() {
			panic!("Endpoint ID can't be empty.")
		}
		
		let has_id_existed = !ids.insert(end_conf.id.clone());
		if has_id_existed {
			panic!("Duplicate endpoint IDs were found, endpoint IDs must be unique.");
		}

		adapters.push(
			create_corresponding_output_endpoint(cmp_reg.clone(), &end_conf)
		);		
	}

	adapters
}

/// Configures the program state according to the configuration file.
/// Caution: This function with panic in case of encountering any errors while trying to read and set the program up according to it,
/// as in any case the program is not intended to be ran in case of a faulty configuration file.
pub fn init_with_file_config(path: &str) -> PState {
	log::debug!("Reading the configuration file at '{path}'.");
	
	let mut raw_config = String::new();
	
	if let Ok(mut file) = File::open(path) {
		let read_res = file.read_to_string(&mut raw_config);
		if let Err(e) = read_res {
			panic!("Failed to read the configuration file at '{path}' due to io error '{e}'.");
		}
	}

	let config: FileConfig =
		match toml::from_str(&raw_config) {
			Ok(v) => v,
			Err(_) => panic!("Failed to parse '{path}'.")
		};
	
	let cmp_reg = Arc::new(Mutex::new(create_composition_registry(&config.composition)));

	let out_adapters = create_endpoints(cmp_reg.clone(), &config.endpoints);

	PState {
		remote_addr: config.api_addr
			.unwrap_or(DEFAULT_REMOTE_ADDR.into())
			.parse()
			.expect("Failed to parse field 'api_addr' in the configuration file."),
		cmp_reg,
		adapters: out_adapters,
		
	}
}