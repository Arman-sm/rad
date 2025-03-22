use std::collections::{BTreeSet, VecDeque};

use super::seg_mgr::SegmentPileID;

pub struct SegmentInfo {
    starting_point: u32, // In frames
    length: usize, // In samples
    buf_start_idx: usize,
}

pub struct StaticMemoryLayer {
    buf: Vec<f32>,
    order: VecDeque<SegmentInfo>,
    // disk_layer: 
    designated_buf_size: usize,
    lookup_table: BTreeSet<SegmentInfo>,
}

impl StaticMemoryLayer {
    pub fn new(buf_size: usize) -> Self {
        StaticMemoryLayer {
            buf: Vec::with_capacity(buf_size),
            order: VecDeque::new(),
            designated_buf_size: buf_size,
            lookup_table: BTreeSet::new(),
        }
    }

    fn buf_size(&self) -> usize {
        self.designated_buf_size.max(self.buf.len())
    }

    pub fn push_segment(&mut self, pile_id: SegmentPileID) {
        let SegmentPileID(pile_id) = pile_id;

        // 
        let free_from_idx = match self.order.front() {
            Some(seg_info) => seg_info.buf_start_idx + seg_info.length,
            None => 0
        };
        
        let free_until_idx = match self.order.back() {
            Some(seg_info) => if seg_info.buf_start_idx < free_from_idx { self.buf_size() } else { seg_info.buf_start_idx },
            None => self.buf_size()
        };

        let remaining_space = last_buf
    }
}