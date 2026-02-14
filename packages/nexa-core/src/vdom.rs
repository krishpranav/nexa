use slotmap::{SlotMap, new_key_type};
use smallvec::SmallVec;

new_key_type! {
    pub struct NodeId;
}

#[derive(Default)]
pub struct GenericArena<T> {
    items: SlotMap<NodeId, T>,
}

impl<T> GenericArena<T> {
    pub fn new() -> Self {
        Self {
            items: SlotMap::with_key(),
        }
    }
    pub fn insert(&mut self, item: T) -> NodeId {
        self.items.insert(item)
    }
    pub fn get(&self, id: NodeId) -> Option<&T> {
        self.items.get(id)
    }
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.items.get_mut(id)
    }
}

pub struct VDomArena {
    pub nodes: GenericArena<VirtualNode>,
    pub metadata: GenericArena<NodeMetadata>,
}

#[derive(Default, Clone, Copy)]
pub struct NodeMetadata {
    pub is_static: bool,
    pub render_count: u64,
}

impl VDomArena {
    pub fn new() -> Self {
        Self {
            nodes: GenericArena::new(),
            metadata: GenericArena::new(),
        }
    }

    pub fn insert_with_metadata(&mut self, node: VirtualNode, metadata: NodeMetadata) -> NodeId {
        let id = self.nodes.insert(node);
        // Ensure metadata arena stays in sync
        self.metadata.items.insert_with_key(|_| metadata);
        id
    }
}

pub enum VirtualNode {
    Element(Element),
    Text(Text),
    Fragment(Fragment),
    Component(Component),
    Suspense(Suspense),
    Placeholder,
}

pub struct Element {
    pub tag: &'static str,
    pub props: SmallVec<[Attribute; 4]>,
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
    pub key: Option<String>,
}

pub struct Attribute {
    pub name: &'static str,
    pub value: String,
}

pub struct Text {
    pub text: String,
    pub parent: Option<NodeId>,
}

pub struct Fragment {
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

pub struct Component {
    pub name: &'static str,
    pub render_fn: fn() -> NodeId,
    pub scope: Option<crate::runtime::ScopeId>,
    pub parent: Option<NodeId>,
}

pub struct Suspense {
    pub fallback: NodeId,
    pub actual: NodeId,
    pub parent: Option<NodeId>,
}

use std::cell::RefCell;

thread_local! {
    static ACTIVE_ARENA: RefCell<Option<*mut VDomArena>> = RefCell::new(None);
}

/// Sets the active arena for the current thread.
/// # Safety
/// The caller must ensure the pointer is valid for the duration of the closure
/// and that no other mutable references exist.
pub unsafe fn set_active_arena<F, R>(arena: &mut VDomArena, f: F) -> R
where
    F: FnOnce() -> R,
{
    ACTIVE_ARENA.with(|ptr| {
        let old = *ptr.borrow();
        *ptr.borrow_mut() = Some(arena as *mut _);
        let res = f();
        *ptr.borrow_mut() = old;
        res
    })
}

pub fn get_active_arena<F, R>(f: F) -> R
where
    F: FnOnce(&mut VDomArena) -> R,
{
    ACTIVE_ARENA.with(|ptr| {
        if let Some(raw) = *ptr.borrow() {
            unsafe { f(&mut *raw) }
        } else {
            panic!("No active VDOM arena! Are you calling rsx! outside a Runtime context?");
        }
    })
}
