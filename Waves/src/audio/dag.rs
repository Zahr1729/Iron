// This file is for the effect dag to compute the audio data

use std::sync::Arc;

use crate::audio::effects::{Effect, Zero};

pub struct EffectDAG {
    root_index: usize,
    nodes: Vec<Arc<dyn Effect>>,
}

impl EffectDAG {
    pub fn new(root_index: usize, nodes: Vec<Arc<dyn Effect>>) -> Self {
        Self { root_index, nodes }
    }

    pub fn is_empty(&self) -> bool {
        (self.nodes.len() == 0)
    }

    // The static here means the Effect can live forever (ie it is not a reference ((mostly)))
    pub fn add_effect(&mut self, effect: impl Effect + 'static) -> Arc<dyn Effect> {
        let e = Arc::new(effect);
        self.add_arc_effect(e)
    }

    pub fn add_arc_effect(&mut self, effect: Arc<dyn Effect>) -> Arc<dyn Effect> {
        self.nodes.push(effect.clone());
        effect
    }

    fn root(&self) -> Arc<dyn Effect> {
        assert!(self.nodes.len() > 0, "DAG has no elements");
        self.nodes[self.root_index].clone()
    }

    pub fn root_index(&self) -> usize {
        self.root_index
    }

    pub fn nodes(&self) -> &[Arc<dyn Effect>] {
        &self.nodes
    }

    pub fn set_root_index(&mut self, root_index: usize) {
        self.root_index = root_index;
    }
}

impl Effect for EffectDAG {
    fn apply(&self, output: &mut [f32], start_sample: usize, channels: usize) {
        self.root().apply(output, start_sample, channels);
    }
}
