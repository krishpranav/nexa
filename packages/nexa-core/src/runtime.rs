use crate::mutations::Mutation;
use crate::vdom::{NodeId, VDomArena, VirtualNode};
use slotmap::{new_key_type, SlotMap};
use std::collections::HashMap;

new_key_type! {
    pub struct ScopeId;
}

pub trait Scheduler {
    fn schedule_update(&self);
}

// Default no-op scheduler for testing/initialization
pub struct NoOpScheduler;
impl Scheduler for NoOpScheduler {
    fn schedule_update(&self) {}
}

pub struct Runtime<S: Scheduler = NoOpScheduler> {
    pub arena: VDomArena,
    pub scopes: SlotMap<ScopeId, Scope>,
    pub mutation_buffer: Vec<Mutation>,
    pub scheduler: S,
    pub component_registry: HashMap<&'static str, fn() -> NodeId>,
    pub root_node: Option<NodeId>,
}

pub struct Scope {
    pub id: ScopeId,
    pub name: String,
}

impl<S: Scheduler> Runtime<S> {
    pub fn new(scheduler: S) -> Self {
        Self {
            arena: VDomArena::new(),
            scopes: SlotMap::with_key(),
            mutation_buffer: Vec::new(),
            scheduler,
            component_registry: HashMap::new(),
            root_node: None,
        }
    }

    pub fn mount(&mut self, root_component_name: &'static str, root_fn: fn() -> NodeId) {
        self.component_registry.insert(root_component_name, root_fn);

        // Create root scope
        let scope_id = self.scopes.insert(Scope {
            id: ScopeId::default(), // Will be overwritten by insert, but we need structure
            name: "Root".to_string(),
        });

        // Initial render
        let root_id = (root_fn)();
        self.root_node = Some(root_id);

        // Record initial mutation
        self.mutation_buffer.push(Mutation::PushRoot {
            id: root_id.data().as_ffi(),
        });
    }

    pub fn update(&mut self) {
        // In the future this will drain the dirty queue from scheduler
        // and re-render dirty scopes.
        // For now, it's a placeholder for the update loop.
        self.scheduler.schedule_update();
    }

    pub fn drain_mutations(&mut self) -> Vec<Mutation> {
        let mut mutations = Vec::new();
        std::mem::swap(&mut self.mutation_buffer, &mut mutations);
        mutations
    }
}
