use std::io::Read;

use dasp_sample::Sample;
use minimp3::{Decoder, Frame};

use crate::source::{iter_func, utils::delay_iter, Src};

pub fn minimp3_src(mut file: std::fs::File) -> Src {
	let mut decoder = Decoder::new(file);

	let first_frame = 
		match decoder.next_frame() {
			Ok(frame) => {
				frame
			},
			Err(e) => panic!("{}", e)
		};

	let func = Box::new(
		move || {
			match decoder.next_frame() {
				Ok(Frame { data, .. }) => {
					Some(data.iter().map(|s| Sample::from_sample(*s)).collect())
				},
				Err(minimp3::Error::Eof) => {
					None
				},
				Err(e) => panic!("{:?}", e)
			}
		}
	);

	Src::new(iter_func(
		delay_iter(func, Some(first_frame.data.iter().map(|s| Sample::from_sample(*s)).collect()))),
		first_frame.sample_rate as usize,
		first_frame.channels
	)
}