use slotmap::new_key_type;
use smallvec::SmallVec;

new_key_type! {
    pub struct NodeId;
}

pub enum ReactiveNode {
    Signal(SignalNode),
    Computed(ComputedNode),
    Effect(EffectNode),
}

pub struct SignalNode {
    pub subscribers: SmallVec<[NodeId; 4]>,
}

pub struct ComputedNode {
    pub dependencies: SmallVec<[NodeId; 4]>,
    pub subscribers: SmallVec<[NodeId; 4]>,
    pub depth: usize,
}

pub struct EffectNode {
    pub dependencies: SmallVec<[NodeId; 4]>,
    pub depth: usize,
}
