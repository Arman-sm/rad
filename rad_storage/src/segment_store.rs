use std::{collections::BTreeSet, path::PathBuf, sync::{Arc, LazyLock, Mutex, RwLock}};

static STORE_ID_COUNTER: LazyLock<Arc<Mutex<u8>>> = LazyLock::new(|| Arc::new(Mutex::new(0)));

pub struct HeapID(pub PathBuf);

/// It's and index to quantify how recent a segment was accessed relative to other segments
type TRecencyIdx = i32;

/// It's an id specific to each `SegmentStore` instance and is in place so that it is ensured that the right segment store is being called.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PileID(u16, StoreID);

pub enum SegmentData {
    Cache(Box<[f32]>),
    Mem(Box<[f32]>),
}

impl SegmentData {
    fn len(&self) -> usize {
        match self {
            Self::Cache(c) => c.len(),
            Self::Mem(m) => m.len(),
        }
    }

    pub fn fetch(&self) -> &[f32] {
        match self {
            Self::Cache(c) => &c,
            Self::Mem(m) => &m,
        }
    }
}

pub struct Segment {
    pub frame_idx: u64,
    pub data: SegmentData,
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

const DEFAULT_CACHE_LIMIT_BYTES: u64 = 32 * 1024 * 1024; // 32MB

pub struct SegmentStore {
    mem_segments: Vec<(PileID, u64)>,
    cache_size: u64,
    cache_limit_bytes: u64,
    piles: Vec<BTreeSet<Segment>>,
    store_id: StoreID,
    /// Contains a record of all segments of the current segment store in memory and is sorted by their recency.
    recency_set: BTreeSet<(TRecencyIdx, PileID, u64)>,
}

impl SegmentStore {
    pub fn new() -> Arc<RwLock<Self>> {
        let store = SegmentStore {
            mem_segments: Vec::new(),
            cache_limit_bytes: DEFAULT_CACHE_LIMIT_BYTES,
            cache_size: 0,
            piles: Vec::new(),
            recency_set: Default::default(),
            store_id: Default::default(),
        };

        Arc::new(RwLock::new(store))
    }

    fn drop_cache_segment(&mut self, pile_id: PileID, frame_idx: u64, recency_idx: i32) {
        assert!(self.store_id == pile_id.1);

        let key = Segment {
            frame_idx,
            data: SegmentData::Cache(Box::new([])),
            recency_idx,
            channels: 0,
        };

        let seg = self.piles[pile_id.0 as usize].take(&key);

        if let Some(seg_data) = seg {
            if !matches!(seg_data.data, SegmentData::Cache(_)) {
                log::error!("An attempt was made to drop a non-cache segment.");
                self.piles[pile_id.0 as usize].insert(seg_data);
                return;
            }
            
            self.cache_size -= (seg_data.data.len() * size_of::<f32>()) as u64;
            self.recency_set.remove(&(recency_idx, pile_id, frame_idx));
            
        } else {
            log::error!("A drop request was initiated for a nonexistent cache segment.");
        }
    }

    fn shake_cache(&mut self) {
        while self.cache_limit_bytes < self.cache_size {
            let (recency_idx, pile_id, frame_idx) = self.recency_set.first().unwrap().clone(); 
            self.drop_cache_segment(pile_id, frame_idx, recency_idx);
        }
    }

    pub fn new_pile_id(&mut self) -> PileID {
        self.piles.push(BTreeSet::new());
        
        PileID(self.piles.len() as u16 - 1, self.store_id)
    }

    fn new_recency_idx(&self) -> TRecencyIdx {
        match self.recency_set.last() {
            Some((last_access_idx, ..)) => last_access_idx + 1,
            None => 0
        }
    }

    pub fn insert(&mut self, pile_id: PileID, frame_idx: u64, channels: u8, data: Box<[f32]>, permanent: bool) {
        assert!((data.len() * size_of::<f32>()) as u64 <= self.cache_limit_bytes);
        assert!(pile_id.1 == self.store_id);

        if self.find(pile_id, frame_idx).is_some() {
            log::warn!("Tried to add intersecting audio segments.");
            return;
        }

        let recency_idx = self.new_recency_idx();
        let data_size_bytes = data.len() * size_of::<f32>();

        let seg = Segment {
            frame_idx,
            data: if permanent { SegmentData::Mem(data) } else { SegmentData::Cache(data) },
            recency_idx,
            channels,
        };
        
        self.piles[pile_id.0 as usize].insert(seg);

        if permanent {
            self.mem_segments.push((pile_id, frame_idx));
        } else {
            self.recency_set.insert((recency_idx, pile_id, frame_idx));
            self.cache_size += data_size_bytes as u64;
        }

        // TODO: Address the possibility of the new segment being removed as well.
        self.shake_cache();
    }

    pub fn find(&mut self, pile_id: PileID, frame_idx: u64) -> Option<&Segment> {
        assert!(pile_id.1 == self.store_id);

        let end_seg = Segment {
            frame_idx,
            data: SegmentData::Cache(Box::new([])),
            recency_idx: 0,
            channels: 0,
        };

        let mut rng = self.piles[pile_id.0 as usize].range(..=end_seg);

        match rng.next_back() {
            Some(seg) => {
                if seg.frames() + seg.frame_idx <= frame_idx {
                    return None;
                }
                
                if matches!(seg.data, SegmentData::Cache(_)) {
                    // Update the recency set
                    self.recency_set.remove(&(seg.recency_idx, pile_id, seg.frame_idx));
                    self.recency_set.insert((self.new_recency_idx(), pile_id, seg.frame_idx));
                }

                Some(seg)
            },
            None => None,
        }
    }
}