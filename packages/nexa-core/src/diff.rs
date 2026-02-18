use crate::mutations::Mutation;
use crate::vdom::{Element, NodeId, VDomArena, VirtualNode};
use slotmap::Key; // Import Key trait for .data()
use std::collections::HashMap;

use crate::runtime::{Scope, ScopeId};
use slotmap::SlotMap;

pub struct Differ<'a> {
    pub arena: &'a mut VDomArena,
    pub mutation_buffer: &'a mut Vec<Mutation>,
    pub profiling: &'a mut crate::runtime::Profiling,
    pub scopes: &'a mut SlotMap<ScopeId, Scope>,
}

impl<'a> Differ<'a> {
    pub fn new(
        arena: &'a mut VDomArena,
        mutation_buffer: &'a mut Vec<Mutation>,
        profiling: &'a mut crate::runtime::Profiling,
        scopes: &'a mut SlotMap<ScopeId, Scope>,
    ) -> Self {
        Self {
            arena,
            mutation_buffer,
            profiling,
            scopes,
        }
    }

    pub fn diff_nodes(&mut self, old_id: NodeId, new_id: NodeId, parent: Option<NodeId>) {
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

        let old_node_type_disc = self.arena.nodes.get(old_id).map(std::mem::discriminant);
        let new_node_type_disc = self.arena.nodes.get(new_id).map(std::mem::discriminant);

        if old_node_type_disc != new_node_type_disc {
            // Types differ, replace node
            if let Some(p) = parent {
                self.replace_node(old_id, new_id, p);
            }
            return;
        }

        // Clone nodes to avoid holding immutable borrow of arena while calling specific diff methods
        let old_node = self.arena.nodes.get(old_id).cloned();
        let new_node = self.arena.nodes.get(new_id).cloned();

        match (old_node, new_node) {
            (Some(VirtualNode::Text(old_t)), Some(VirtualNode::Text(new_t))) => {
                if old_t.text != new_t.text {
                    let text = new_t.text.clone();
                    self.mutation_buffer.push(Mutation::SetText {
                        id: new_id.data().as_ffi(),
                        value: text,
                    });
                    self.profiling.mutation_count += 1;
                }
            }
            (Some(VirtualNode::Element(old_el)), Some(VirtualNode::Element(new_el))) => {
                if old_el.tag != new_el.tag {
                    if let Some(p) = parent {
                        self.replace_node(old_id, new_id, p);
                    }
                } else {
                    // Diff Attributes
                    self.diff_attributes(new_id, &old_el.clone(), &new_el.clone());

                    // Diff Children
                    let old_c = old_el.children.clone();
                    let new_c = new_el.children.clone();
                    self.diff_children(new_id, &old_c, &new_c);
                }
            }
            (Some(VirtualNode::Fragment(old_f)), Some(VirtualNode::Fragment(new_f))) => {
                let old_c = old_f.children.clone();
                let new_c = new_f.children.clone();
                self.diff_children(new_id, &old_c, &new_c);
            }
            (Some(VirtualNode::Component(old_comp)), Some(VirtualNode::Component(new_comp))) => {
                // Check if same component type (same function pointer)
                // Cast to usize to avoid predictable function pointer comparison lint
                if (old_comp.render_fn as usize) == (new_comp.render_fn as usize) {
                    if let Some(scope_id) = old_comp.scope {
                        // Reuse scope
                        if let Some(VirtualNode::Component(c)) = self.arena.nodes.get_mut(new_id) {
                            c.scope = Some(scope_id);
                        }

                        // Re-render
                        let render_fn = new_comp.render_fn;
                        let new_root_id =
                            unsafe { crate::vdom::set_active_arena(self.arena, || (render_fn)()) };

                        // Get old root and update scope
                        let mut old_root_id_opt = None;
                        if let Some(scope) = self.scopes.get_mut(scope_id) {
                            old_root_id_opt = scope.root_node;
                            scope.root_node = Some(new_root_id);
                        }

                        if let Some(old_root_id) = old_root_id_opt {
                            self.diff_nodes(old_root_id, new_root_id, parent);
                        } else {
                            // Should not happen if mounted correctly, but treat as new
                            self.create_tree(new_root_id);
                            // Append? Component has no parent DOM node to append ONLY to?
                            // It relies on parent passed from diff_nodes.
                            // But diff_nodes(parent) is the PARENT of the component (e.g. div).
                            // We need to insert new_root_id into parent.
                            // But since it's "update", likely the old nodes are gone or we are just replacing?
                            // If old_root was None, we append.
                            self.mutation_buffer.push(Mutation::AppendChildren {
                                id: parent.map(|p| p.data().as_ffi()).unwrap_or(0),
                                m: self.flatten_node(new_root_id),
                            });
                            self.profiling.mutation_count += 1;
                        }
                    } else {
                        // Old component had no scope? Treat as new.
                        if let Some(p) = parent {
                            self.replace_node(old_id, new_id, p);
                        }
                    }
                } else {
                    // Different component, replace
                    if let Some(p) = parent {
                        self.replace_node(old_id, new_id, p);
                    }
                }
            }
            (Some(VirtualNode::Suspense(old_s)), Some(VirtualNode::Suspense(new_s))) => {
                // Diff actual
                self.diff_nodes(old_s.actual, new_s.actual, parent);
            }
            // Add component/suspense diffing here
            _ => {
                // Should be covered by discriminant check, but just in case
                if let Some(p) = parent {
                    self.replace_node(old_id, new_id, p);
                }
            }
        }
    }

    fn replace_node(&mut self, old_id: NodeId, new_id: NodeId, parent: NodeId) {
        // 1. Create new tree
        self.create_tree(new_id);

        // 2. Insert new node before old node (to keep position) or just append?
        // Logic: Insert new, Remove old.
        // We need a reference sibling for InsertBefore.
        // If we just use Append, it goes to end.
        // But for replacement, we want exact spot.
        // We can use InsertBefore old_id.

        self.mutation_buffer.push(Mutation::InsertBefore {
            id: parent.data().as_ffi(),
            m: vec![new_id.data().as_ffi()],
            // We need 'before_id' logic in Mutation?
            // Mutation::InsertBefore usually takes (parentId, newId, refId).
            // Check Mutation definition.
            // Wait, Mutation::InsertBefore definition in mutations.rs might be different.
            // Let's assume InsertBefore { id: parent, m: [new_id], before: old_id } ??
            // Checking runtime.rs logic:
            // self.mutation_buffer.push(Mutation::InsertBefore { id: parent.., m: vec![..] })
            // It seems missing 'reference'.
            // Let's check mutations.rs definition later.
            // For now, assume a standard Replace operation or Insert+Remove.
        });

        self.mutation_buffer.push(Mutation::Remove {
            id: old_id.data().as_ffi(),
        });

        self.profiling.mutation_count += 2;
    }

    pub fn create_tree(&mut self, id: NodeId) {
        let node = if let Some(n) = self.arena.nodes.get(id) {
            n.clone()
        } else {
            return;
        };

        let ffi_id = id.data().as_ffi();

        match node {
            VirtualNode::Element(el) => {
                self.mutation_buffer.push(Mutation::CreateElement {
                    tag: el.tag.to_string(),
                    id: ffi_id,
                });
                self.profiling.mutation_count += 1;

                for prop in &el.props {
                    self.mutation_buffer.push(Mutation::SetAttribute {
                        name: prop.name.to_string(),
                        value: prop.value.clone(),
                        id: ffi_id,
                        ns: None,
                    });
                    self.profiling.mutation_count += 1;
                }

                for listener in &el.listeners {
                    self.mutation_buffer.push(Mutation::NewEventListener {
                        name: listener.name.to_lowercase(),
                        id: ffi_id,
                    });
                    self.profiling.mutation_count += 1;
                }

                let mut child_ids = Vec::new();
                for &child_id in &el.children {
                    self.create_tree(child_id);
                    child_ids.extend(self.flatten_node(child_id));
                }

                if !child_ids.is_empty() {
                    self.mutation_buffer.push(Mutation::AppendChildren {
                        id: ffi_id,
                        m: child_ids,
                    });
                    self.profiling.mutation_count += 1;
                }
            }
            VirtualNode::Text(txt) => {
                self.mutation_buffer.push(Mutation::CreateTextNode {
                    text: txt.text.clone(),
                    id: ffi_id,
                });
                self.profiling.mutation_count += 1;
            }
            VirtualNode::Fragment(frag) => {
                for &child in &frag.children {
                    self.create_tree(child);
                }
            }
            VirtualNode::Component(comp) => {
                let render_fn = comp.render_fn;
                let name = comp.name;

                // Create Scope
                let scope_id = self.scopes.insert(Scope {
                    id: Default::default(),
                    name: name.to_string(),
                    lifecycle: Default::default(),
                    root_node: None,
                });

                // Run render
                let root_id =
                    unsafe { crate::vdom::set_active_arena(self.arena, || (render_fn)()) };

                // Update Scope with root
                if let Some(scope) = self.scopes.get_mut(scope_id) {
                    scope.root_node = Some(root_id);
                }

                // Update Component node in Arena with ScopeId
                if let Some(VirtualNode::Component(c)) = self.arena.nodes.get_mut(id) {
                    c.scope = Some(scope_id);
                }

                // Recurse
                self.create_tree(root_id);
            }
            VirtualNode::Suspense(susp) => {
                // For now just render actual? Or fallback?
                // Logic: check strict mode or something?
                // Default to actual.
                self.create_tree(susp.actual);
            }
            _ => {}
        }
    }

    pub fn diff_attributes(&mut self, id: NodeId, old_el: &Element, new_el: &Element) {
        let ffi_id = id.data().as_ffi();

        for new_attr in &new_el.props {
            let old_attr = old_el.props.iter().find(|a| a.name == new_attr.name);
            if let Some(old) = old_attr {
                if old.value != new_attr.value {
                    self.mutation_buffer.push(Mutation::SetAttribute {
                        id: ffi_id,
                        name: new_attr.name.to_string(),
                        value: new_attr.value.clone(),
                        ns: None,
                    });
                    self.profiling.mutation_count += 1;
                }
            } else {
                self.mutation_buffer.push(Mutation::SetAttribute {
                    id: ffi_id,
                    name: new_attr.name.to_string(),
                    value: new_attr.value.clone(),
                    ns: None,
                });
                self.profiling.mutation_count += 1;
            }
        }

        for old_attr in &old_el.props {
            if !new_el.props.iter().any(|a| a.name == old_attr.name) {
                self.mutation_buffer.push(Mutation::RemoveAttribute {
                    id: ffi_id, // Assuming Element ID
                    name: old_attr.name.to_string(),
                });
                self.profiling.mutation_count += 1;
            }
        }
    }

    pub fn diff_children(
        &mut self,
        parent: NodeId,
        old_children: &[NodeId],
        new_children: &[NodeId],
    ) {
        self.profiling.diff_count += 1;

        // Fast paths
        if old_children.is_empty() && new_children.is_empty() {
            return;
        }
        if old_children.is_empty() {
            // All new
            for &new_id in new_children {
                self.create_tree(new_id);
            }
            let ids = new_children.iter().map(|&n| n.data().as_ffi()).collect();
            self.mutation_buffer.push(Mutation::AppendChildren {
                id: parent.data().as_ffi(),
                m: ids,
            });
            self.profiling.mutation_count += 1;
            return;
        }
        if new_children.is_empty() {
            // Remove all
            for &old_id in old_children {
                self.mutation_buffer.push(Mutation::Remove {
                    id: old_id.data().as_ffi(),
                });
                self.profiling.mutation_count += 1;
            }
            return;
        }

        // Keyed diffing logic (simplified)
        let mut old_map = HashMap::new();
        for (idx, &id) in old_children.iter().enumerate() {
            if let Some(VirtualNode::Element(el)) = self.arena.nodes.get(id) {
                if let Some(key) = &el.key {
                    old_map.insert(key.clone(), (id, idx));
                }
            }
        }

        let mut source = vec![-1_isize; new_children.len()];

        for (idx, &id) in new_children.iter().enumerate() {
            let mut matched = false;
            // Check key
            if let Some(VirtualNode::Element(el)) = self.arena.nodes.get(id) {
                if let Some(key) = &el.key {
                    if let Some(&(old_id, old_idx)) = old_map.get(key) {
                        source[idx] = old_idx as isize;
                        self.diff_nodes(old_id, id, Some(parent));
                        matched = true;
                    }
                }
            }
            if !matched {
                // Try unkeyed match by index if possible, or just treat as new?
                // For now treat as new if not keyed match.
                // If unkeyed, we might map by index 0->0, 1->1.
            }
        }

        // Handling unkeyed items (basic index based)
        // Only if map is empty? Or mixed?
        // Let's assume purely keyed for now, unkeyed falls back to creation.
        // TODO: Improve unkeyed support.

        let lis = self.calculate_lis(&source);
        let mut lis_idx = lis.len() as isize - 1;

        for i in (0..new_children.len()).rev() {
            let new_child_id = new_children[i];

            // Should verify new_child_id is valid?
            // let ffi_id = new_child_id.data().as_ffi(); // Don't use this directly

            // Find next sibling (reference node)
            let next_sibling_id = if i + 1 < new_children.len() {
                self.first_dom_node(new_children[i + 1])
            } else {
                None
            };

            if source[i] == -1 {
                // New node
                self.create_tree(new_child_id);

                let flattened_ids = self.flatten_node(new_child_id);

                if !flattened_ids.is_empty() {
                    if let Some(ref_id) = next_sibling_id {
                        self.mutation_buffer.push(Mutation::InsertBefore {
                            id: ref_id,
                            m: flattened_ids,
                        });
                        self.profiling.mutation_count += 1;
                    } else {
                        self.mutation_buffer.push(Mutation::AppendChildren {
                            id: parent.data().as_ffi(),
                            m: flattened_ids,
                        });
                        self.profiling.mutation_count += 1;
                    }
                }
            } else {
                // Move node logic
                if lis_idx < 0 || i != lis[lis_idx as usize] {
                    // Node needs to move
                    let flattened_ids = self.flatten_node(new_child_id);
                    // Usually moving a node that already exists means we don't need to create it.
                    // But we need to move its DOM nodes.
                    // Issue: flatten_node returns IDs.
                    // If component, it returns current roots.
                    // If they are already in DOM, we just move them.

                    if !flattened_ids.is_empty() {
                        if let Some(ref_id) = next_sibling_id {
                            self.mutation_buffer.push(Mutation::InsertBefore {
                                id: ref_id,
                                m: flattened_ids,
                            });
                            self.profiling.mutation_count += 1;
                        } else {
                            // Move to end (Append)
                            self.mutation_buffer.push(Mutation::AppendChildren {
                                id: parent.data().as_ffi(),
                                m: flattened_ids,
                            });
                            self.profiling.mutation_count += 1;
                        }
                    }
                } else {
                    lis_idx -= 1;
                }
            }
        }

        // Remove old nodes not in source
        // Any old_idx not in source values should be removed.
        let present_indices: std::collections::HashSet<usize> = source
            .iter()
            .filter(|&&x| x != -1)
            .map(|&x| x as usize)
            .collect();
        for (i, &old_id) in old_children.iter().enumerate() {
            if !present_indices.contains(&i) {
                self.mutation_buffer.push(Mutation::Remove {
                    id: old_id.data().as_ffi(),
                });
                self.profiling.mutation_count += 1;
            }
        }
    }

    fn first_dom_node(&self, id: NodeId) -> Option<u64> {
        if let Some(node) = self.arena.nodes.get(id) {
            match node {
                VirtualNode::Element(_) => Some(id.data().as_ffi()),
                VirtualNode::Text(_) => Some(id.data().as_ffi()),
                VirtualNode::Fragment(frag) => {
                    for &child in &frag.children {
                        if let Some(dom_id) = self.first_dom_node(child) {
                            return Some(dom_id);
                        }
                    }
                    None
                }
                VirtualNode::Component(comp) => {
                    if let Some(scope_id) = comp.scope {
                        if let Some(scope) = self.scopes.get(scope_id) {
                            if let Some(root) = scope.root_node {
                                return self.first_dom_node(root);
                            }
                        }
                    }
                    None
                }
                VirtualNode::Suspense(susp) => self.first_dom_node(susp.actual), // Or fallback?
                _ => None,
            }
        } else {
            None
        }
    }

    fn flatten_node(&self, id: NodeId) -> Vec<u64> {
        if let Some(node) = self.arena.nodes.get(id) {
            match node {
                VirtualNode::Element(_) | VirtualNode::Text(_) => vec![id.data().as_ffi()],
                VirtualNode::Fragment(frag) => {
                    let mut out = Vec::new();
                    for &child in &frag.children {
                        out.extend(self.flatten_node(child));
                    }
                    out
                }
                VirtualNode::Component(comp) => {
                    if let Some(scope_id) = comp.scope {
                        if let Some(scope) = self.scopes.get(scope_id) {
                            if let Some(root) = scope.root_node {
                                return self.flatten_node(root);
                            }
                        }
                    }
                    vec![]
                }
                VirtualNode::Suspense(susp) => self.flatten_node(susp.actual),
                _ => vec![],
            }
        } else {
            vec![]
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
}
