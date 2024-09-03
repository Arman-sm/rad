use std::{sync::{Arc, Mutex}, thread::ThreadId};

use crate::{composition::TWrappedCompositionState, compositor::{init_compositor_thread, CompositionBufferNode}};

pub enum CompositorState<const BUF_SIZE: usize> {
    Active(ThreadId, Arc<CompositionBufferNode<BUF_SIZE>>),
    Killed
}

pub struct CompositorData<const BUF_SIZE: usize> {
    sample_rate: u32,
    cmp_id: String,
    state: Arc<Mutex<CompositorState<BUF_SIZE>>>
}

impl<const BUF_SIZE: usize> CompositorData<BUF_SIZE> {
    pub fn new(cmp_id: String, sample_rate: u32, state: Arc<Mutex<CompositorState<BUF_SIZE>>>) -> Self {
        CompositorData {
            cmp_id,
            sample_rate,
            state,
        }
    }
}

pub struct CompositionRegistry<const BUF_SIZE: usize> {
    compositions: Vec<TWrappedCompositionState>,
    compositors: Vec<CompositorData<BUF_SIZE>>,
}

impl<const BUF_SIZE: usize> CompositionRegistry<BUF_SIZE> {
    pub fn new() -> Self {
        CompositionRegistry {
            compositions: Vec::new(),
            compositors: Vec::new(),
        }
    }

    pub fn get_active_buf(&mut self, cmp_id: &str, sample_rate: u32) -> Option<Arc<CompositionBufferNode<BUF_SIZE>>> {
        for cmp in self.compositors.iter_mut() {
            let cmp_lock = cmp.state.lock().unwrap();

            if let CompositorState::Active(_, ref buf) = *cmp_lock {
                if cmp.cmp_id == cmp_id && cmp.sample_rate == sample_rate {
                    return Some(buf.clone());
                }
            }

        }

        let cmp_state = self.compositions.iter().find(|d| d.read().unwrap().get_id() == cmp_id)?.clone();

        let (compositor, node) = init_compositor_thread::<BUF_SIZE>(sample_rate, cmp_state);

        self.compositors.push(compositor);

        Some(node)
    }
    
    pub fn find_composition(&self, cmp_id: &str) -> Option<&TWrappedCompositionState> {
        self.compositions.iter().find(|c| c.read().unwrap().id == cmp_id )
    }

    // pub fn find_compositor(&self, cmp_id: &str, sample_rate: u32) -> Option<&CompositorData<BUF_SIZE>> {
    //     self.compositors.iter().find(|d| d.cmp_id == cmp_id && d.sample_rate == sample_rate)
    // }

    pub fn push_composition(&mut self, cmp: TWrappedCompositionState) {
        self.compositions.push(cmp);
    }
}