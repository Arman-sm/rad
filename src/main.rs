use file_config::init_with_file_config;
use dbg_cli::start_dbg_cli;
use rad_remote::start_remote_server;

mod file_config;
mod arg_config;
mod dbg_cli;

#[tokio::main]
async fn main() {
    // Configurations and settings set by command line arguments
    let arg_config = arg_config::get_arg_config();
    
    // Logger configuration:
	simple_logger::init_with_level(arg_config.log_level).unwrap();

    if !arg_config.file_config_path.exists() {
        panic!("Configuration file at '{}' does not exist.", arg_config.file_config_path.display())
    }

    // Setting things up using the file configuration.
    let mut state = init_with_file_config(arg_config.file_config_path.as_os_str().to_str().unwrap());

    if arg_config.dbg_cli {
        // Compact cli controller for debugging:
        start_dbg_cli(&arg_config, &mut state);
        return;
    }

    start_remote_server(state.cmp_reg, state.adapters, state.remote_addr).await.unwrap();
}