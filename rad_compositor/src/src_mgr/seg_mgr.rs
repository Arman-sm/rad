use std::collections::BTreeSet;

use crate::source::utils::SampleBuf;

use super::memory_layer::MemoryLayer;

#[derive(PartialEq)]
struct SegmentPriorityData {
    last_used: u32,
}

impl PartialOrd for SegmentPriorityData {
    fn ge(&self, other: &Self) -> bool {
        self.last_used >= other.last_used
    }

    fn gt(&self, other: &Self) -> bool {
        self.last_used > other.last_used
    }

    fn le(&self, other: &Self) -> bool {
        self.last_used <= other.last_used
    }

    fn lt(&self, other: &Self) -> bool {
        self.last_used < other.last_used
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.last_used.partial_cmp(&other.last_used)
    }
}

pub enum MemoryLimit {
    NoLimit,
    Bytes(u32),
}

pub enum MemoryStrategy {
    Dynamic(MemoryLimit),
    // TODO
    // / Allocates a block of memory with the specified size and puts all the buffers in it but allocates dynamically if needed.
    // SoftStatic(u32),
    // / Allocates a block of memory with the specified size and puts all the buffers in it.
    // HardStatic(u32),
}

pub struct SegmentPileID(pub u32);
struct SegmentInfo {
    seg_pile_id: u32,
    starting_point: u32, // In frames
    length: u32, // In frames
    in_memory: bool,
    on_disk: bool,
}

pub struct SegmentManager {
    memory_strategy: MemoryStrategy,
    memory_layer: MemoryLayer,
    priority_tree: BTreeSet<SegmentPriorityData>,
}

impl SegmentManager {
    pub fn new(mem_limit: MemoryLimit) -> Self {
        SegmentManager {
            memory_strategy: MemoryStrategy::Dynamic(mem_limit),
            memory_layer: MemoryLayer::new(),
            priority_tree: BTreeSet::new(),
        }
    }

    pub fn push_segment(&mut self, seg_pile_id: &SegmentPileID, buf: SampleBuf) {
        self.segment_piles
    }
}