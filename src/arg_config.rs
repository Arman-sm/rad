use std::path::PathBuf;

use clap::Parser;

#[cfg(target_family = "unix")]
pub const DEFAULT_CONFIG_FILE_PATH: &str = "/etc/rad/rad.conf";

// ! The feature of saving audios has been removed temporarily
// This is where the server would keep its saved audios
#[cfg(target_family = "unix")]
pub const DEFAULT_DATA_DIR:          &str = "/var/lib/rad";

// TODO: Test for windows
#[cfg(target_family = "windows")]
pub const DEFAULT_CONFIG_FILE_PATH: &str = "%program_data%\\rad\\rad.conf";
#[cfg(target_family = "windows")]
pub const DEFAULT_DATA_DIR:          &str = "%program_data%\\rad\\data";

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ArgConfig {
    #[clap(short = 'd', long = "data-dir", env = "RAD_DATA_DIR", default_value = DEFAULT_DATA_DIR)]
    data_dir: PathBuf,

	#[clap(short = 'D', long = "enable-dbg-cli", default_value_t = false)]
	pub dbg_cli: bool,
	
	#[clap(short = 'c', long = "config", default_value = DEFAULT_CONFIG_FILE_PATH)]
	pub config_path: PathBuf,
}

impl ArgConfig {
	pub fn audio_dir(&self) -> PathBuf { self.data_dir.join("audios") }
}

pub fn get_arg_config() -> ArgConfig {
	let conf = ArgConfig::parse();

	if !conf.data_dir.exists() {
		log::warn!("Data directory '{}' doesn't exist", conf.data_dir.display());

		match std::fs::create_dir_all(&conf.data_dir) {
			Ok(_) => log::info!("Successfully created the data directory"),
			Err(e) => {
				panic!("Couldn't create the data directory: {}", e);
			}
		}
    }

	if !conf.audio_dir().exists() {
		log::warn!("Audio directory '{}' doesn't exist", conf.data_dir.display());

		match std::fs::create_dir_all(conf.data_dir.join("audios")) {
			Ok(_) => log::info!("Successfully created the audios directory"),
			Err(e) => {
				panic!("Couldn't create the audios directory: {}", e);
			}
		}
	}

	conf
}

