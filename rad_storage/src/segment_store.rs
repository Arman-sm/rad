use std::{collections::BTreeSet, sync::{Arc, LazyLock, Mutex}};

static STORE_ID_COUNTER: LazyLock<Arc<Mutex<u8>>> = LazyLock::new(|| Arc::new(Mutex::new(0)));

/// It's and index to quantify how recent a segment was accessed relative to other segments
type TRecencyIdx = i32;

/// It's an id specific to each `SegmentStore` instance and is in place so that it is ensured that the right segment store is being called.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreID(u8);

impl StoreID {
    fn new() -> StoreID {
        let mut lk = STORE_ID_COUNTER.lock().unwrap();

        *lk += 1;
        StoreID(*lk)
    }
}

impl Default for StoreID {
    fn default() -> Self {
        StoreID::new()
    }
}

/// It's an id specific to each source using an specific segment store and is used to distinctively identify segments of each source.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PileID(u16, StoreID);

pub struct Segment {
    pub frame_idx: u64,
    pub data: Box<[f32]>,
    recency_idx: TRecencyIdx,
    pub channels: u8,
}

impl Segment {
    pub fn frames(&self) -> u64 {
        self.data.len() as u64 / self.channels as u64
    }
}

impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.frame_idx == other.frame_idx
    }
}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.frame_idx.partial_cmp(&other.frame_idx)
    }
}

impl Eq for Segment {}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Default)]
pub struct SegmentStore {
    piles: Vec<BTreeSet<Segment>>,
    /// Pile id counter used to allocate new piles.
    pile_cnt: u16,
    store_id: StoreID,
    /// Contains a record of all segments of the current segment store in memory and is sorted by their recency.
    recency_set: BTreeSet<(TRecencyIdx, PileID, u32)>,
}

impl SegmentStore {
    pub fn new() -> Self {
        SegmentStore::default()
    }

    pub fn new_pile_id(&mut self) -> PileID {
        self.piles.push(BTreeSet::new());
        
        self.pile_cnt += 1;
        PileID(self.pile_cnt-1, self.store_id)
    }

    fn new_recency_idx(&self) -> TRecencyIdx {
        match self.recency_set.last() {
            Some((last_access_idx, ..)) => last_access_idx + 1,
            None => 0
        }
    }

    pub fn insert(&mut self, pile_id: PileID, frame_idx: u64, channels: u8, data: Box<[f32]>) {
        assert!(pile_id.1 == self.store_id);
        
        let recency_idx = self.new_recency_idx();
        
        let seg = Segment {
            frame_idx,
            data,
            recency_idx,
            channels,
        };
        
        self.piles[pile_id.0 as usize].insert(seg);
    }

    pub fn find(&self, pile_id: PileID, frame_idx: u64) -> Option<&Segment> {
        assert!(pile_id.1 == self.store_id);

        let end_seg = Segment {
            frame_idx,
            data: Box::new([]),
            recency_idx: 0,
            channels: 0
        };

        let mut rng = self.piles[pile_id.0 as usize].range(..=end_seg);

        match rng.next_back() {
            Some(seg) => {
                if seg.frames() + seg.frame_idx <= frame_idx {
                    return None;
                }
                
                // // Update the recency set
                // self.recency_set.insert((self.new_recency_idx(), pile_id, seg.frame_idx));
                // self.recency_set.remove(&(seg.recency_idx, pile_id, seg.frame_idx));

                Some(seg)
            },
            None => None,
        }
    }
}