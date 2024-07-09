use std::{fs::{read_dir, File}, path::{Path, PathBuf}};

use crate::sources::symphonia::init_symphonia_src;

use super::{iter::{IterSrcFunc, TIterSrcFuncReturn}, queue::QueueSrc};

pub const DEFAULT_HINT_EXT: &str = "mp3";

pub fn audio_mime_subtype_from_ext(ext: &str) -> &str {
	match ext  {
		"mp3" => "mpeg",
		"wave" => "wav",
		any => any
	}
}

fn find_files(path: &Path, depth: u8) -> Option<Vec<PathBuf>> {
	let mut files = Vec::new();
	
	let dir_iter = read_dir(path).ok()?;
	for item in dir_iter {
		let item = item.ok()?;
		if depth != 0 && item.file_type().ok()?.is_dir() {
			let mut sub_files = find_files(item.path().as_path(), depth - 1)?;
			files.append(&mut sub_files);
		} else {
			files.push(item.path());
		}
	}

	Some(files)
}

pub fn queue_from_directory(path: &Path, sample_rate: u32, depth: u8) -> Option<QueueSrc> {
	let mut queue = QueueSrc::new(sample_rate);	
	let file_paths = find_files(path, depth)?;
	for file_path in file_paths {
		log::debug!("Reading '{}'", file_path.display());
		let ext: &str = match file_path.extension() {
			Some(ext) => { ext.try_into().unwrap_or(DEFAULT_HINT_EXT) },
			None => DEFAULT_HINT_EXT
		};
		let mime_type = format!("audio/{}", audio_mime_subtype_from_ext(ext));
		let file = File::open(file_path).ok()?;

		let src = init_symphonia_src(file, &mime_type).ok()?;

		queue.push(src.into());
	}
		
	Some(queue)
}

pub fn delay_iter(mut func: Box<IterSrcFunc>, mut first_call_return: TIterSrcFuncReturn) -> Box<IterSrcFunc> {
	let mut is_first_call = true;
	
	Box::new(move || {
		if is_first_call { 
			is_first_call = false;
			return std::mem::take(&mut first_call_return);
		}

		func()
	})
}