use crate::diff::Differ;
use crate::mutations::Mutation;
use crate::scheduler::Scheduler;
use crate::vdom::{NodeId, VDomArena, VirtualNode, set_active_arena};
use nexa_signals::NodeType;
use nexa_signals::dependency::{allocate_node, execute, pop_observer, push_observer, take_dirty};

use slotmap::{Key, SlotMap, new_key_type};
use std::collections::HashMap;

new_key_type! {
    pub struct ScopeId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    Begin,
    Diff,
    Commit,
}

#[derive(Default, Debug, Clone)]
pub struct Profiling {
    pub render_count: u64,
    pub diff_count: u64,
    pub mutation_count: u64,
}

pub struct Runtime<S: Scheduler> {
    pub arena: VDomArena,
    pub scopes: SlotMap<ScopeId, Scope>,
    pub mutation_buffer: Vec<Mutation>,
    pub scheduler: S,
    pub component_registry: HashMap<&'static str, fn() -> NodeId>,
    pub root_fn: Option<fn() -> NodeId>,
    pub root_effect: Option<nexa_signals::SignalId>,
    pub root_node: Option<NodeId>,
    pub phase: RenderPhase,
    pub profiling: Profiling,
}

pub struct Scope {
    pub id: ScopeId,
    pub name: String,
    pub lifecycle: ComponentLifecycle,
    pub root_node: Option<NodeId>,
}

#[derive(Default)]
pub struct ComponentLifecycle {
    pub on_mount: Option<fn()>,
    pub on_update: Option<fn()>,
    pub on_drop: Option<Option<fn()>>, // Double option for "has drop fn" vs "is it set"
}

impl<S: Scheduler> Runtime<S> {
    pub fn new(scheduler: S) -> Self {
        Self {
            arena: VDomArena::new(),
            scopes: SlotMap::with_key(),
            mutation_buffer: Vec::new(),
            scheduler,
            component_registry: HashMap::new(),
            root_fn: None,
            root_effect: None,
            root_node: None,
            phase: RenderPhase::Begin,
            profiling: Profiling::default(),
        }
    }

    pub fn mount(&mut self, root_component_name: &'static str, root_fn: fn() -> NodeId) {
        tracing::info!(
            "Runtime::mount started for component: {}",
            root_component_name
        );
        self.phase = RenderPhase::Begin;
        self.profiling.render_count += 1;

        self.component_registry.insert(root_component_name, root_fn);
        self.root_fn = Some(root_fn);

        // Create a signal for the root effect/computation
        let effect_id = allocate_node(NodeType::Effect);
        self.root_effect = Some(effect_id);

        let _scope_id = self.scopes.insert(Scope {
            id: ScopeId::default(),
            name: root_component_name.to_string(),
            lifecycle: ComponentLifecycle::default(),
            root_node: None,
        });

        // Initial render via run_root
        self.run_root();

        tracing::info!(
            "Mount complete. Generated {} mutations.",
            self.profiling.mutation_count
        );
    }

    fn run_root(&mut self) {
        if let Some(root_fn) = self.root_fn {
            tracing::debug!("Running root render...");

            // Track dependencies
            if let Some(effect_id) = self.root_effect {
                push_observer(effect_id);
            }

            let root_id = unsafe { set_active_arena(&mut self.arena, || (root_fn)()) };

            // Stop tracking
            if self.root_effect.is_some() {
                pop_observer();
            }

            self.phase = RenderPhase::Commit;

            if let Some(old_root) = self.root_node {
                // Diff against old root
                Differ::new(
                    &mut self.arena,
                    &mut self.mutation_buffer,
                    &mut self.profiling,
                    &mut self.scopes,
                )
                .diff_nodes(old_root, root_id, None);
            } else {
                // Initial creation
                // PushRoot to set the root ID context
                self.mutation_buffer.push(Mutation::PushRoot {
                    id: root_id.data().as_ffi(),
                });

                Differ::new(
                    &mut self.arena,
                    &mut self.mutation_buffer,
                    &mut self.profiling,
                    &mut self.scopes,
                )
                .create_tree(root_id);

                // Append the new root to container
                // We need to flatten to find actual element IDs (skip fragments/components)
                let roots = self.flatten_children(&[root_id]);
                if !roots.is_empty() {
                    self.mutation_buffer.push(Mutation::AppendChildren {
                        id: 0, // Container
                        m: roots,
                    });
                    self.profiling.mutation_count += 1;
                }
            }

            self.root_node = Some(root_id);
        }
    }

    // ... update ...

    pub fn update(&mut self) {
        self.phase = RenderPhase::Begin;

        // 1. Gather dirty signals
        let dirty = take_dirty();

        if dirty.is_empty() {
            return;
        }

        self.profiling.render_count += 1;

        // 2. Schedule
        self.scheduler.schedule(dirty);

        // 3. Run Scheduler
        self.phase = RenderPhase::Diff;
        let queue = nexa_signals::dependency::GRAPH.with(|g| {
            let graph = g.borrow();
            self.scheduler.run(&graph)
        });

        // Execute signal updates
        nexa_signals::dependency::execute(queue.clone());

        for sig in queue {
            // Re-render components dependent on sig
            if Some(sig) == self.root_effect {
                tracing::info!("Root effect dirty, re-rendering...");
                self.run_root();
            }
        }

        for scope in self.scopes.values_mut() {
            if let Some(on_update) = scope.lifecycle.on_update {
                on_update();
            }
        }

        self.phase = RenderPhase::Commit;
        // Batching/Draining happens in drain_mutations
    }

    pub fn flatten_fragment(&self, id: NodeId, output: &mut Vec<NodeId>) {
        if let Some(VirtualNode::Fragment(frag)) = self.arena.nodes.get(id) {
            for &child in &frag.children {
                self.flatten_fragment(child, output);
            }
        } else {
            output.push(id);
        }
    }

    pub fn drain_mutations(&mut self) -> Vec<Mutation> {
        let mut mutations = Vec::new();
        std::mem::swap(&mut self.mutation_buffer, &mut mutations);
        mutations
    }

    pub fn handle_event(&mut self, node_id: u64, event_name: &str, event: crate::events::Event) {
        // use slotmap::Key;
        // Reconstruct NodeId from u64 (assuming 1:1 mapping with ffi_id logic)
        // Helper: NodeId::from(Data::from_ffi(node_id))
        // But NodeId key type details are hidden by slotmap macro?
        // Actually NodeId is new_key_type, so we need to construct it carefully.
        // nexa_core's NodeId might not be directly constructible from u64 if logic is complex,
        // but slotmap keys are usually (version, index).
        // Wait, ffi_id = id.data().as_ffi().
        // We need to reverse this.
        let id = NodeId::from(slotmap::KeyData::from_ffi(node_id));

        tracing::debug!("Runtime handling event '{}' for node {:?}", event_name, id);

        let mut callback_to_run = None;

        if let Some(VirtualNode::Element(el)) = self.arena.nodes.get(id) {
            for listener in &el.listeners {
                if listener.name == event_name {
                    callback_to_run = Some(listener.cb.clone());
                    break;
                }
            }
        } else {
            // Maybe it's a component root or something?
            // Or maybe the node was removed?
            tracing::warn!("Event targeted at missing or non-element node {:?}", id);
        }

        if let Some(cb) = callback_to_run {
            (cb.borrow_mut())(event);
            self.update(); // Trigger reactivity update after event
        }
    }

    pub fn flatten_children(&self, children: &[NodeId]) -> Vec<u64> {
        let mut out = Vec::new();
        for &id in children {
            if let Some(VirtualNode::Fragment(frag)) = self.arena.nodes.get(id) {
                out.extend(self.flatten_children(&frag.children));
            } else {
                out.push(id.data().as_ffi());
            }
        }
        out
    }

    pub fn verify_tree_integrity(&self) {
        if let Some(root) = self.root_node {
            self.walk_verify(root);
        }
    }

    fn walk_verify(&self, id: NodeId) {
        let node = self
            .arena
            .nodes
            .get(id)
            .expect("Orphaned or invalid NodeId detected!");
        match node {
            VirtualNode::Element(el) => {
                for &child in &el.children {
                    self.walk_verify(child);
                }
            }
            VirtualNode::Fragment(frag) => {
                for &child in &frag.children {
                    self.flatten_verify(child);
                }
            }
            VirtualNode::Component(comp) => {
                // components are essentially the start of a subtree
                if let Some(scope_id) = comp.scope {
                    if !self.scopes.contains_key(scope_id) {
                        panic!("Component {} has invalid ScopeId!", comp.name);
                    }
                }
            }
            _ => {}
        }
    }

    fn flatten_verify(&self, id: NodeId) {
        self.walk_verify(id);
    }
}
