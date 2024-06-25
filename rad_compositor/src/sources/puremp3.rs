use std::fs::File;

use puremp3::read_mp3;

use crate::source::{iter_func, Src};

pub fn puremp3_src(file: File) -> Src {
	let (header, iter) = read_mp3(file).expect("Failed to read mp3");

	let mut res = iter.map(|v| {println!("frame"); vec![v.0, v.1]}).collect::<Vec<_>>().into_iter();
	
	Src::new(
		iter_func(Box::new(move || res.next())),
		header.sample_rate.hz() as usize,
		header.channels.num_channels()
	)
}