use slotmap::new_key_type;
use smallvec::SmallVec;

new_key_type! {
    pub struct NodeId;
}

#[derive(Debug, Clone)]
pub enum ReactiveNode {
    Signal(SignalNode),
    Computed(ComputedNode),
    Effect(EffectNode),
}

#[derive(Debug, Clone)]
pub struct SignalNode {
    pub subscribers: SmallVec<[NodeId; 4]>,
}

#[derive(Debug, Clone)]
pub struct ComputedNode {
    pub dependencies: SmallVec<[NodeId; 4]>,
    pub subscribers: SmallVec<[NodeId; 4]>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct EffectNode {
    pub dependencies: SmallVec<[NodeId; 4]>,
    pub depth: usize,
}
