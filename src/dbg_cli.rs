use std::{fs::{canonicalize, File}, io::{stdin, stdout, Write}, path::PathBuf};

use rad_compositor::{composition::TWrappedCompositionState, sources::symphonia::init_symphonia_src};

use crate::{arg_config::ArgConfig, file_config::PState};

const ASCII_START_BANNER: &str = "
  \x1b[31;214m█▀█ ▄▀█ █▀▄   \x1b[93m█▀▄ █▄▄ █▀▀   \x1b[38;5;250m█▀▀ █   █
  \x1b[31;214m█▀▄ █▀█ █▄▀   \x1b[93m█▄▀ █▄█ █▄█   \x1b[38;5;250m█▄▄ █▄▄ █\x1b[0m";

// TODO: Change this to a list of the commands and their description and format it later in the program
const HELP_PAGE: &str =
"> op [{filepath} | +{relative to {data-dir}/...}] -> Opens a new audio file
> sc {cmp-id} | set-cmp {cmp-id}                  -> Selects the composition for use with other commands
> amp                                             -> Outputs amplification of the selected composition
> amp {amp}                                       -> Changes amplification of the selected composition
> ap lst | adapter list                           -> Lists the adapters
> ap del {ap-id}                                  -> Deletes an adapter by ID
> t | time                                        -> Time value of a composition in seconds
> go {time(second)}                               -> Sets timeline value
> help                                            -> Prints the help page";

const SECOND_DECIMAL_PRECISION: u8 = 1;
pub fn format_f32_sec(seconds: f32) -> String {
	const PRECISION_POW: f32 = 10u16.pow(SECOND_DECIMAL_PRECISION as u32) as f32;

	((seconds * PRECISION_POW).floor() / PRECISION_POW).to_string()
}
	
pub fn init_dbg_cli(run_conf: &ArgConfig, p_state: &mut PState) {
	let PState { composition_states: ref mut cmp_states, ref mut adapters } = p_state;
	let mut curr_cmp: Option<TWrappedCompositionState> = None;
	
	let mut stdout = stdout();
	let stdin = stdin();

	let mut cmd = String::new();

	println!("{}\n", ASCII_START_BANNER);
	
	loop {
		print!("\x1b[1m\x1b[38;5;214m » \x1b[0m\x1b[22m");
		stdout.flush().unwrap();
		
		cmd.clear();
		stdin.read_line(&mut cmd).unwrap();
		// Removes the excess '\n' at the end of the line
		cmd.remove(cmd.len() - 1);

		if cmd.is_empty() {
			continue;
		}
		
		match *cmd.trim_end().split(" ").collect::<Vec<_>>() {
			["help"] => {
				println!("{}", HELP_PAGE);
			},
			// pause / play
			["p"] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				curr_cmp.write().unwrap().is_paused ^= true;
			},
			["ap", "lst"] | ["adapter", "list"] => {
				const IS_CLOSED_TRUE_STR:  &str = "Closed";
				const IS_CLOSED_FALSE_STR: &str = "Open";

				// Hardcoded space of each field in characters:
				// ID(10) | Status(16) | Opened/Closed(8)
				println!("\x1b[0;30m     ID     |      Status      |   Op/C  \x1b[0m");
				for ap in adapters.iter() {
					let is_closed_str = match ap.is_closed() {
						true  => IS_CLOSED_TRUE_STR,
						false => IS_CLOSED_FALSE_STR,
					};

					println!(" {:^10} | {:^16} | {:^8}", ap.id(), ap.status(), is_closed_str)
				}
			},
			["ap", "del", id] => {
				let idx = adapters
					.iter()
					.enumerate()
					.find_map(|(i, ap)| if ap.id() == id { Some(i) } else { None });

				match idx {
					Some(i) => { 
						adapters[i].close();
						adapters.remove(i);
					},
					None => eprintln!("No adapter was found with the specified ID")
				};
			},
			["amp"] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				println!("{}", curr_cmp.read().unwrap().amplification);
			},
			["amp", _amp] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				match _amp.parse::<f32>() {
					Ok(amp) => {
						curr_cmp.write().unwrap().amplification = amp;
					},
					Err(_) => {
						eprintln!("Invalid value for second");
						continue;
					}
				};
			},
			// Selects a composition for later use with other commands.
			["sc", id] | ["set-cmp", id] => {
				let c = cmp_states
					.iter()
					.find_map(|c| if c.read().unwrap().id == id { Some(c.clone()) } else { None });
				
				if c.is_none() {
					eprintln!("No composition exists with this ID.");
					continue;
				}

				curr_cmp = c;
			},
			// Opens a file as a source and adds it to the selected composition
			["op", file_path] | ["open", file_path] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};
				
				let file_path =
					if file_path.starts_with("+") {
						run_conf.audio_dir().join(file_path.strip_prefix("+").unwrap())
					} else { PathBuf::from(file_path) };
				
				if !file_path.exists() || !file_path.is_file() { eprintln!("File does not exist."); continue; }
				
				log::debug!("Opening \"{:?}\"", canonicalize(file_path.clone()).unwrap());
				let file = File::open(file_path).unwrap();
				
				log::debug!("Initializing the source");
				let src = match init_symphonia_src(file) {
					Ok(_src) => _src,
					Err(err) => {
						eprintln!("Failed to create the source.");
						log::error!("Initialization failed with error \"{:?}\".", err);

						continue;
					}
				};
				curr_cmp.write().unwrap().push_src_default(src)
			},
			["go", _sec] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				if let Ok(sec) = _sec.parse::<f32>() {
					curr_cmp.write().unwrap().set_time_millis((sec * 1000.0) as u64)
				} else {
					eprintln!("Invalid value for second");
					continue;
				};
			},
			["t"] | ["time"] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				let cmp = curr_cmp.read().unwrap();
				println!("{}", format_f32_sec(cmp.start_t.elapsed().as_f64() as f32))
			},
			_ => println!("Invalid command")
		}
	}
}