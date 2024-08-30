use std::{env, fs::{canonicalize, File}, io::{stdin, stdout, Write}, path::PathBuf};

use rad_compositor::{composition::TWrappedCompositionState, source::utils::{audio_mime_subtype_from_ext, queue_from_directory}, sources::symphonia::init_symphonia_src};

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
> help                                            -> Prints the help page
> p                                               -> Pauses/Plays the selected composition";

const SEC_F32_DECIMAL_PRECISION: u8 = 2;
fn format_f32_sec(seconds: f32) -> String {
	const PRECISION_POW: f32 = 10u16.pow(SEC_F32_DECIMAL_PRECISION as u32) as f32;

	((seconds * PRECISION_POW).floor() / PRECISION_POW).to_string()
}

const QUEUE_SAMPLE_RATE: u32 = 44100;
const OPEN_DIR_SEARCH_DEPTH: u8 = u8::MAX;
const DEFAULT_HINT_EXT: &str = "mp3";

pub fn start_dbg_cli(run_conf: &ArgConfig, p_state: &mut PState) {
	let PState { composition_states: ref mut cmp_states, ref mut adapters } = p_state;
	// The composition state selected by the `sc` command
	let mut curr_cmp: Option<TWrappedCompositionState> = None;
	
	let mut stdout = stdout();
	let stdin = stdin();

	// The command at hand
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
			["op", path] | ["open", path] => {
				let curr_cmp = match &curr_cmp {
					None => { eprintln!("Please select a composition first."); continue; },
					Some(cmp) => cmp
				};

				let path = path.trim_start().trim_end();
				
				if path.is_empty() {
					eprintln!("No path was given.");
					continue;
				}

				let path = match path.chars().next().unwrap() {
					'+' => {
						run_conf.audio_dir().join(path.strip_prefix("+").unwrap())
					},
					'~' => {
						if cfg!(unix) {
							let home_dir = env::var_os("HOME");
							
							let without_prefix = match path.strip_prefix("~/") {
								Some(path) => path,
								None => {
									// $HOME + "" = $HOME
									if path.len() == 1 {
										""
									} else {
										eprintln!("Invalid path");
										continue;
									}
								}
							};

							match home_dir {
								Some(home_dir) => PathBuf::from(home_dir).join(without_prefix),
								None => PathBuf::from(path),
							}
						} else {
							PathBuf::from(path)
						}
					},
					_ => {
						PathBuf::from(path)
					}
				};
			
				if !path.exists() { eprintln!("File does not exist."); continue; }
				
				if path.is_dir() {
					log::debug!("Directory detected, making a queue.");
					let queue = match queue_from_directory(&path, QUEUE_SAMPLE_RATE, OPEN_DIR_SEARCH_DEPTH) {
						Some(queue) => queue,
						None => { eprintln!("Something went wrong while creating a queue."); continue; }
					};

					curr_cmp.write().unwrap().push_src_default(queue.into());

					continue;
				}
	
				log::debug!("Opening '{:?}'", canonicalize(&path).unwrap());
				let file = File::open(&path).unwrap();
				
				log::debug!("Initializing the source");
				let ext: &str = match path.extension() {
					Some(ext) => { ext.try_into().unwrap_or(DEFAULT_HINT_EXT) },
					None => DEFAULT_HINT_EXT
				};

				let mime_subtype = audio_mime_subtype_from_ext(ext);

				let src = match init_symphonia_src(file, format!("audio/{mime_subtype}").as_str()) {
					Ok(_src) => _src,
					Err(err) => {
						eprintln!("Failed to create the source.");
						log::error!("Initialization failed with error '{:?}'.", err);

						continue;
					}
				};
				curr_cmp.write().unwrap().push_src_default(src.into())
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