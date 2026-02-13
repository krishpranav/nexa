use crate::mutations::Mutation;
use crate::vdom::{NodeId, VDomArena};
use nexa_scheduler::Scheduler;

use slotmap::{new_key_type, Key, SlotMap};
use std::collections::HashMap;

new_key_type! {
    pub struct ScopeId;
}

pub struct Runtime {
    pub arena: VDomArena,
    pub scopes: SlotMap<ScopeId, Scope>,
    pub mutation_buffer: Vec<Mutation>,
    pub scheduler: Scheduler,
    pub component_registry: HashMap<&'static str, fn() -> NodeId>,
    pub root_node: Option<NodeId>,
}

pub struct Scope {
    pub id: ScopeId,
    pub name: String,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            arena: VDomArena::new(),
            scopes: SlotMap::with_key(),
            mutation_buffer: Vec::new(),
            scheduler: Scheduler::new(),
            component_registry: HashMap::new(),
            root_node: None,
        }
    }

    pub fn mount(&mut self, root_component_name: &'static str, root_fn: fn() -> NodeId) {
        self.component_registry.insert(root_component_name, root_fn);

        let _scope_id = self.scopes.insert(Scope {
            id: ScopeId::default(),
            name: "Root".to_string(),
        });

        // Initial render
        let root_id = (root_fn)();
        self.root_node = Some(root_id);

        self.mutation_buffer.push(Mutation::PushRoot {
            id: root_id.data().as_ffi(),
        });
    }

    pub fn update(&mut self) {
        // 1. Gather dirty signals from TLS
        // Accessing thread-local graph from nexa-signals
        let dirty = nexa_signals::context::GRAPH.with(|g| g.borrow_mut().take_dirty());

        if dirty.is_empty() {
            return;
        }

        // 2. Schedule
        self.scheduler.schedule(dirty);

        // 3. Run Scheduler
        nexa_signals::context::GRAPH.with(|g| {
            let graph = g.borrow();
            let queue = self.scheduler.run(&graph);

            // 4. Re-render affected components
            for sig in queue {
                // In a real implementation: look up ScopeId for SignalId
                tracing::trace!("Signal {:?} updated, re-rendering dependents", sig);
            }
        });
    }

    pub fn drain_mutations(&mut self) -> Vec<Mutation> {
        let mut mutations = Vec::new();
        std::mem::swap(&mut self.mutation_buffer, &mut mutations);
        mutations
    }
}
