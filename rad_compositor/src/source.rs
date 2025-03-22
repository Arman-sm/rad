use formatted::FormattedStreamSource;
use queue::QueueSrc;

pub mod utils;
pub mod queue;
pub mod formatted;

pub type TSample = f32;
pub type TFrameIdx = u64;

// TODO: Comments!
/// This trait contains the methods required for implementing a new source.
pub trait BaseSource {
    /// It returns the sample-rate which the source is outputting.
    /// 
    /// **Note: The returned value will stay the same over every call.**
    fn sample_rate(&self) -> TFrameIdx;
    
    /// Returns the duration which the audio is going to end after in number of frames *if it was know or available with minimal or no computation*.
    /// If the duration is absolutely needed then `current_duration_frames` should be used instead.
    ///
    /// **Note: If if is known that the source never ends the value 0 will be returned and not None.**
    fn duration(&self) -> Option<TFrameIdx>;
    
    /// Like `duration` returns the duration which the audio is going to last in number of frames.
    /// The only difference is that no matter the computational cost it will figure the duration out and return it.
    /// 
    /// **Note: The returned value will stay the same over every call.**
    fn current_duration_frames(&self) -> TFrameIdx;
    
    fn get_by_frame_i(&mut self, frame_idx: TFrameIdx) -> Option<Vec<TSample>>;
}

/// A type for staying generic over different types of sources.
pub enum Source {
    File(formatted::FormattedStreamSource),
    Queue(queue::QueueSrc)
}

impl BaseSource for Source {
    fn get_by_frame_i(&mut self, frame_i: TFrameIdx) -> Option<Vec<TSample>> {
        match self {
            Self::File(file) => file.get_by_frame_i(frame_i),
            Self::Queue(queue) => queue.get_by_frame_i(frame_i)
        }
    }

    fn current_duration_frames(&self) -> TFrameIdx {
        match self {
            Self::File(file) => file.current_duration_frames(),
            Self::Queue(queue) => queue.current_duration_frames()
        }
    }

    fn duration(&self) -> Option<TFrameIdx> {
        match self {
            Self::File(file) => file.duration(),
            Self::Queue(queue) => queue.duration()
        }
    }

    fn sample_rate(&self) -> TFrameIdx {
        match self {
            Self::File(file) => file.sample_rate(),
            Self::Queue(queue) => queue.sample_rate(),
        }
    }
}

impl From<FormattedStreamSource> for Source {
    fn from(value: FormattedStreamSource) -> Self { Source::File(value) }
}

impl From<QueueSrc> for Source {
    fn from(value: QueueSrc) -> Self { Source::Queue(value) }
}