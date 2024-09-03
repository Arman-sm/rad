use std::collections::LinkedList;

use crate::compositor::approximate_frame_linear;

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
        let mut offset: usize = 0;
        for src in self.sources.iter_mut() {
            let frame = if src.sample_rate() == self.sample_rate {
                src.get_by_frame_i(frame_i - offset)
            } else {
                approximate_frame_linear(src, self.sample_rate, frame_i - offset, 0)
            };
            
            if frame.is_some() { return frame; }
            
            offset += src.duration().unwrap();
        }

        None
    }
}