use file_config::init_with_file_config;
use dbg_cli::start_dbg_cli;

mod file_config;
mod arg_config;
mod dbg_cli;

fn main() {
    // Configurations and settings set by command line arguments
    let arg_config = arg_config::get_arg_config();
    
	simple_logger::init_with_level(arg_config.log_level).unwrap();

    if !arg_config.file_config_path.exists() {
        panic!("Configuration file at '{}' does not exist.", arg_config.file_config_path.display())
    }
    let mut state = init_with_file_config(arg_config.file_config_path.as_os_str().to_str().unwrap());

    // Compact and temporary cli controller for debugging:
    start_dbg_cli(&arg_config, &mut state);
}
