use crate::vdom::VDomArena;
use slotmap::new_key_type;
use std::cell::RefCell;
use std::collections::HashMap;

new_key_type! {
    pub struct ScopeId;
}

pub struct Scope {
    pub id: ScopeId,
    pub parent_id: Option<ScopeId>,
    pub height: u32,
    pub name: String,
    pub hook_idx: RefCell<usize>,
    pub hooks: RefCell<Vec<Box<dyn std::any::Any>>>, // Simplified hook storage
    pub context: RefCell<HashMap<String, Box<dyn std::any::Any>>>,
}

impl Scope {
    pub fn new(id: ScopeId, name: String, parent: Option<&Scope>) -> Self {
        let height = parent.map(|p| p.height + 1).unwrap_or(0);
        Self {
            id,
            parent_id: parent.map(|p| p.id),
            height,
            name,
            hook_idx: RefCell::new(0),
            hooks: RefCell::new(Vec::new()),
            context: RefCell::new(HashMap::new()),
        }
    }

    pub fn reset_hook_idx(&self) {
        *self.hook_idx.borrow_mut() = 0;
    }
}

pub struct Runtime {
    pub scopes: slotmap::SlotMap<ScopeId, Scope>,
    pub arena: VDomArena,
    pub root_scope: Option<ScopeId>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scopes: slotmap::SlotMap::with_key(),
            arena: VDomArena::new(),
            root_scope: None,
        }
    }

    pub fn create_scope(&mut self, name: String, parent_id: Option<ScopeId>) -> ScopeId {
        // In a real impl, we'd lookup parent to get height, etc.
        // For now, minimal impl involves double lookup or internal mutation if we hold ref.
        // We'll just insert.
        self.scopes.insert_with_key(|id| {
            let height = if let Some(_p_id) = parent_id {
                // Warning: direct lookup in same map while inserting is tricky if borrowing.
                // SlotMap insert_with_key usually gives just key.
                // We'll fix height later or use 0 for now.
                // To do this properly we need to get parent first, data, then insert.
                0
            } else {
                0
            };

            Scope {
                id,
                parent_id,
                height,
                name,
                hook_idx: RefCell::new(0),
                hooks: RefCell::new(Vec::new()),
                context: RefCell::new(HashMap::new()),
            }
        })
    }

    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id)
    }
}
