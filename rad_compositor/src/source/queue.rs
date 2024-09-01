use std::collections::LinkedList;

use super::{BaseSource, Source, TSample};

pub struct QueueSrc {
    sources: LinkedList<Source>,
    sample_rate: u32
}

impl QueueSrc {
    pub fn new(sample_rate: u32) -> Self {
        QueueSrc {
            sources: LinkedList::new(),
            sample_rate
        }
    }

    pub fn push(&mut self, src: Source) {
        self.sources.push_back(src);
    }

    pub fn pop(&mut self) -> Option<Source> {
        self.sources.pop_back()
    }

    pub fn sources(&self) -> &LinkedList<Source> {
        &self.sources
    }

    pub fn sources_mut(&mut self) -> &mut LinkedList<Source> {
        &mut self.sources
    }
}

impl BaseSource for QueueSrc {
    fn sample_rate(&self) -> u32 { self.sample_rate }

    fn current_duration_frames(&self) -> usize {
        let mut dur = 0;
        
        for src in self.sources.iter() {
            dur += src.current_duration_frames();
        }

        dur
    }

    fn duration(&self) -> Option<usize> {
        let mut dur = 0;

        for src in self.sources.iter() {
            match src.duration() {
                Some(d) => dur += d,
                None => { return None; }
            }
        }

        Some(dur)
    }

    fn get_by_frame_i(&mut self, frame_i: usize) -> Option<Vec<TSample>> {
        let mut offset = 0;
        for src in self.sources.iter_mut() {
            let frame = src.get_by_frame_i(frame_i - offset);
            if frame.is_some() { return frame; }
            
            offset += src.duration().unwrap();
        }

        println!("a");
        None
    }
}