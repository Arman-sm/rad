
use file::FileSource;
use iter::IterSrc;
use queue::QueueSrc;

pub mod utils;
pub mod queue;
pub mod iter;
pub mod file;

pub type TSample = f32;

pub trait BaseSource {
    fn sample_rate(&self) -> u32;
    fn current_duration_frames(&self) -> usize;
    fn duration(&self) -> Option<usize>;
    fn get_by_frame_i(&mut self, frame_i: usize) -> Option<Vec<TSample>>;
}

pub enum Source {
    Iter(iter::IterSrc),
    File(file::FileSource),
    Queue(queue::QueueSrc)
}

impl BaseSource for Source {
    fn get_by_frame_i(&mut self, frame_i: usize) -> Option<Vec<TSample>> {
        match self {
            Self::File(file) => file.get_by_frame_i(frame_i),
            Self::Iter(iter) => iter.get_by_frame_i(frame_i),
            Self::Queue(queue) => queue.get_by_frame_i(frame_i)
        }
    }

    fn current_duration_frames(&self) -> usize {
        match self {
            Self::File(file) => file.current_duration_frames(),
            Self::Iter(iter) => iter.current_duration_frames(),
            Self::Queue(queue) => queue.current_duration_frames()
        }
    }

    fn duration(&self) -> Option<usize> {
        match self {
            Self::File(file) => file.duration(),
            Self::Iter(iter) => iter.duration(),
            Self::Queue(queue) => queue.duration()
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            Self::File(file) => file.sample_rate(),
            Self::Iter(iter) => iter.sample_rate(),
            Self::Queue(queue) => queue.sample_rate(),
        }
    }
}

impl From<FileSource> for Source {
    fn from(value: FileSource) -> Self { Source::File(value) }
}

impl From<IterSrc> for Source {
    fn from(value: IterSrc) -> Self { Source::Iter(value) }
}

impl From<QueueSrc> for Source {
    fn from(value: QueueSrc) -> Self { Source::Queue(value) }
}