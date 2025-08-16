// This file is for the effect dag to compute the audio data

use std::sync::Arc;

use crate::audio::effects::{Effect, Zero};

pub struct EffectDAG {
    nodes: Vec<Arc<dyn Effect>>,
}

impl EffectDAG {
    pub fn new() -> Self {
        Self {
            nodes: vec![Arc::new(Zero)],
        }
    }

    fn root(&self) -> Arc<dyn Effect> {
        assert!(self.nodes.len() > 0, "DAG has no elements");
        self.nodes[0].clone()
    }
}

impl Effect for EffectDAG {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.root().apply(output, start_sample, channels);
    }
}
