use dasp_sample::{FromSample, Sample};
use super::{IterSrcFunc, TSrcFuncReturn};

pub fn convert_samples_iter<S>(
	mut func: Box<dyn FnMut() -> Option<Vec<S>> + Send + Sync>
) -> Box<IterSrcFunc>
where
	f32: FromSample<S>,
	S: 'static
{
	Box::new(move || {
		let frame: Vec<S> = func()?;
		Some(
			frame.into_iter()
				.map(|v| Sample::from_sample(v))
				.collect::<Vec<f32>>()
		)
	})
}

pub fn delay_iter(mut func: Box<IterSrcFunc>, mut first_call_return: TSrcFuncReturn) -> Box<IterSrcFunc> {
	let mut is_first_call = true;
	
	Box::new(move || {
		if is_first_call { 
			is_first_call = false;
			return std::mem::take(&mut first_call_return);
		}

		func()
	})
}

// pub fn frame_fragment_iter(mut func: Box<IterSrcFunc>, channels: usize) -> Box<IterSrcFunc> {
// 	let mut iter_data = IterData { has_ended: false };
// 	let mut sample_buff = SampleBuff::new(0, call_src_iter(&mut func, &mut iter_data).unwrap());

// 	assert!(sample_buff.len() % channels == 0);
	
// 	let mut frame_c: usize = 0;

// 	Box::new(move || {


// 		Some(res)
// 	})
// }