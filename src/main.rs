use std::thread;

use file_config::init_with_file_config;
use dbg_cli::init_dbg_cli;

mod file_config;
mod arg_config;
mod dbg_cli;

fn main() {
    // TODO: Add ability to set log level via the program args
    
    let log_level =
        if cfg!(debug_assertion) { log::Level::Warn } else { log::Level::Debug };

    simple_logger::init_with_level(log_level).unwrap();

    let arg_config = arg_config::get_arg_config();

    if !arg_config.config_path.exists() {
        panic!("Configuration file at \"{}\" does not exist.", arg_config.config_path.to_str().unwrap())
    }
    let mut p_state = init_with_file_config(arg_config.config_path.as_os_str().to_str().unwrap());

    // Compact and temporary cli controller for debugging:
    if arg_config.dbg_cli {
        thread::Builder::new()
            .name("dbg_cli".to_owned())
            .spawn(move || init_dbg_cli(&arg_config, &mut p_state))
            .unwrap();
    }    
    
    loop {
        
    }
}

