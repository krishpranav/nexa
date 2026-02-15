use crate::mutations::Mutation;
use crate::vdom::{NodeId, VDomArena, VirtualNode, set_active_arena};
use nexa_scheduler::Scheduler;
use nexa_signals::NodeType;
use nexa_signals::context::{allocate_node, pop_observer, push_observer};

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

pub struct Runtime {
    pub arena: VDomArena,
    pub scopes: SlotMap<ScopeId, Scope>,
    pub mutation_buffer: Vec<Mutation>,
    pub scheduler: Scheduler,
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
}

#[derive(Default)]
pub struct ComponentLifecycle {
    pub on_mount: Option<fn()>,
    pub on_update: Option<fn()>,
    pub on_drop: Option<Option<fn()>>, // Double option for "has drop fn" vs "is it set"
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            arena: VDomArena::new(),
            scopes: SlotMap::with_key(),
            mutation_buffer: Vec::new(),
            scheduler: Scheduler::new(),
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
        let effect_id = allocate_node(NodeType::Effect, None);
        self.root_effect = Some(effect_id);

        let _scope_id = self.scopes.insert(Scope {
            id: ScopeId::default(),
            name: root_component_name.to_string(),
            lifecycle: ComponentLifecycle::default(),
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

            // Simple Diff: Remove old root if exists
            if let Some(old_root) = self.root_node {
                tracing::debug!("Removing old root {:?}", old_root);
                self.mutation_buffer.push(Mutation::Remove {
                    id: old_root.data().as_ffi(),
                });
            }

            self.root_node = Some(root_id);

            // PushRoot to set the root ID context
            self.mutation_buffer.push(Mutation::PushRoot {
                id: root_id.data().as_ffi(),
            });

            // Generate creation mutations for the entire tree
            self.generate_initial_tree(root_id);

            // Append the new root to container
            let roots = self.flatten_children(&[root_id]);
            if !roots.is_empty() {
                self.mutation_buffer.push(Mutation::AppendChildren {
                    id: 0, // Container
                    m: roots,
                });
                self.profiling.mutation_count += 1;
            }
        }
    }

    // ... update ...

    pub fn update(&mut self) {
        self.phase = RenderPhase::Begin;

        // 1. Gather dirty signals
        let dirty = nexa_signals::context::GRAPH.with(|g| g.borrow_mut().take_dirty());

        if dirty.is_empty() {
            return;
        }

        self.profiling.render_count += 1;

        // 2. Schedule
        self.scheduler.schedule(dirty);

        // 3. Run Scheduler
        self.phase = RenderPhase::Diff;
        let queue = nexa_signals::context::GRAPH.with(|g| {
            let graph = g.borrow();
            self.scheduler.run(&graph)
        });

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

    /// Keyed diffing algorithm using LIS for move detection
    pub fn diff_children(
        &mut self,
        parent: NodeId,
        old_children: &[NodeId],
        new_children: &[NodeId],
    ) {
        self.profiling.diff_count += 1;

        // Simplified Keyed diffing logic start
        let mut old_map = HashMap::new();
        for (idx, &id) in old_children.iter().enumerate() {
            if let Some(VirtualNode::Element(el)) = self.arena.nodes.get(id) {
                if let Some(key) = &el.key {
                    old_map.insert(key.clone(), (id, idx));
                }
            }
        }

        let mut source = vec![-1_isize; new_children.len()];
        let mut new_map = HashMap::new();

        for (idx, &id) in new_children.iter().enumerate() {
            if let Some(VirtualNode::Element(el)) = self.arena.nodes.get(id) {
                if let Some(key) = &el.key {
                    new_map.insert(key.clone(), idx);
                    if let Some(&(old_id, old_idx)) = old_map.get(key) {
                        source[idx] = old_idx as isize;
                        self.diff_nodes(old_id, id);
                    }
                }
            }
        }

        // Detect and apply moves using LIS
        let lis = self.calculate_lis(&source);
        let mut lis_idx = lis.len() as isize - 1;

        for i in (0..new_children.len()).rev() {
            if source[i] == -1 {
                // New node - should be handled by an Insert mutation
                self.mutation_buffer.push(Mutation::InsertBefore {
                    id: parent.data().as_ffi(),
                    m: vec![new_children[i].data().as_ffi()],
                });
                self.profiling.mutation_count += 1;
            } else {
                if lis_idx < 0 || i != lis[lis_idx as usize] {
                    // Move node
                    self.mutation_buffer.push(Mutation::InsertBefore {
                        id: parent.data().as_ffi(),
                        m: vec![new_children[i].data().as_ffi()],
                    });
                    self.profiling.mutation_count += 1;
                } else {
                    lis_idx -= 1;
                }
            }
        }
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

    pub fn diff_nodes(&mut self, old_id: NodeId, new_id: NodeId) {
        let (is_static, old_count) = {
            let meta = self.arena.metadata.get(new_id).cloned().unwrap_or_default();
            (meta.is_static, meta.render_count)
        };

        if is_static && old_count > 0 {
            return; // Skip diffing static subtree
        }

        self.profiling.diff_count += 1;

        if let Some(meta) = self.arena.metadata.get_mut(new_id) {
            meta.render_count += 1;
        }

        use crate::vdom::VirtualNode::*;
        let old_node = self.arena.nodes.get(old_id);
        let new_node = self.arena.nodes.get(new_id);

        match (old_node, new_node) {
            (Some(Text(old_t)), Some(Text(new_t))) => {
                if old_t.text != new_t.text {
                    self.mutation_buffer.push(Mutation::SetText {
                        id: new_id.data().as_ffi(),
                        value: new_t.text.clone(),
                    });
                    self.profiling.mutation_count += 1;
                }
            }
            (Some(Element(old_el)), Some(Element(new_el))) => {
                if old_el.tag != new_el.tag {
                    // Replace logic...
                } else {
                    // Diff children
                    let old_c = old_el.children.clone();
                    let new_c = new_el.children.clone();
                    self.diff_children(new_id, &old_c, &new_c);
                }
            }
            // Other variants...
            _ => {
                // Replace node
            }
        }
    }

    fn calculate_lis(&self, arr: &[isize]) -> Vec<usize> {
        if arr.is_empty() {
            return vec![];
        }
        let mut p = vec![0; arr.len()];
        let mut m = vec![0; arr.len() + 1];
        let mut l = 0;

        for i in 0..arr.len() {
            if arr[i] == -1 {
                continue;
            }
            let mut lo = 1;
            let mut hi = l;
            while lo <= hi {
                let mid = (lo + hi + 1) / 2;
                if arr[m[mid]] < arr[i] {
                    lo = mid + 1;
                } else {
                    hi = mid - 1;
                }
            }
            let new_l = lo;
            p[i] = m[new_l - 1];
            m[new_l] = i;
            if new_l > l {
                l = new_l;
            }
        }

        let mut res = vec![0; l];
        let mut k = m[l];
        for i in (0..l).rev() {
            res[i] = k;
            k = p[k];
        }
        res
    }

    pub fn drain_mutations(&mut self) -> Vec<Mutation> {
        let mut mutations = Vec::new();
        std::mem::swap(&mut self.mutation_buffer, &mut mutations);
        mutations
    }

    pub fn generate_initial_tree(&mut self, id: NodeId) {
        // Recursively walk the VDOM and generate Create/Append mutations
        let node = if let Some(n) = self.arena.nodes.get(id) {
            n
        } else {
            tracing::error!("Attempted to generate tree for missing node {:?}", id);
            return;
        };

        let ffi_id = id.data().as_ffi();

        match node {
            VirtualNode::Element(el) => {
                // 1. Create Element
                tracing::debug!("Generating initial element: <{}> (id={})", el.tag, ffi_id);
                self.mutation_buffer.push(Mutation::CreateElement {
                    tag: el.tag.to_string(),
                    id: ffi_id,
                });
                self.profiling.mutation_count += 1;

                // 2. Set Attributes
                let props = el.props.clone();
                for prop in props {
                    self.mutation_buffer.push(Mutation::SetAttribute {
                        name: prop.name.to_string(),
                        value: prop.value.clone(),
                        id: ffi_id,
                        ns: None,
                    });
                    self.profiling.mutation_count += 1;
                }

                // 2.5 Attach Listeners
                let listeners = el.listeners.clone();
                for listener in listeners {
                    self.mutation_buffer.push(Mutation::NewEventListener {
                        name: listener.name.to_lowercase(),
                        id: ffi_id,
                    });
                    self.profiling.mutation_count += 1;
                }

                // 3. Recurse children
                let children = el.children.clone();
                let mut child_ids = Vec::new();
                for &child_id in &children {
                    self.generate_initial_tree(child_id);
                    child_ids.push(child_id.data().as_ffi());
                }

                // 4. Append children
                if !child_ids.is_empty() {
                    self.mutation_buffer.push(Mutation::AppendChildren {
                        id: ffi_id,
                        m: child_ids,
                    });
                    self.profiling.mutation_count += 1;
                }
            }
            VirtualNode::Text(txt) => {
                tracing::debug!("Generating initial text: \"{}\" (id={})", txt.text, ffi_id);
                self.mutation_buffer.push(Mutation::CreateTextNode {
                    text: txt.text.clone(),
                    id: ffi_id,
                });
                self.profiling.mutation_count += 1;
            }
            VirtualNode::Fragment(frag) => {
                let children = frag.children.clone();
                for &child in &children {
                    self.generate_initial_tree(child);
                }
            }
            _ => {
                tracing::warn!("Skipping unsupported node type during initial generation");
            }
        }
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
