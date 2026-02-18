use nexa_core::{Attribute, Element, EventListener, NodeId, Runtime, Scheduler, Text, VirtualNode};
use nexa_signals::{Graph, SignalId};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
struct MockScheduler {
    pub scheduled: Rc<RefCell<Vec<SignalId>>>,
}

impl Scheduler for MockScheduler {
    fn schedule(&mut self, dirty: impl IntoIterator<Item = SignalId>) {
        let mut scheduled = self.scheduled.borrow_mut();
        for id in dirty {
            scheduled.push(id);
        }
    }

    fn run(&mut self, _graph: &Graph) -> Vec<SignalId> {
        let mut scheduled = self.scheduled.borrow_mut();
        let executed = scheduled.clone();
        scheduled.clear();
        executed
    }
}

fn create_test_runtime() -> Runtime<MockScheduler> {
    Runtime::new(MockScheduler::default())
}

#[test]
fn test_runtime_initialization() {
    let runtime = create_test_runtime();
    assert!(runtime.root_node.is_none());
}

#[test]
fn test_mount_element() {
    let mut runtime = create_test_runtime();

    fn root_component() -> NodeId {
        use nexa_core::vdom::get_active_arena;
        get_active_arena(|arena| {
            arena.insert(VirtualNode::Element(Element {
                tag: "div",
                props: Default::default(),
                listeners: Default::default(),
                children: Default::default(),
                parent: None,
                key: None,
            }))
        })
    }

    runtime.mount("Root", root_component);

    assert!(runtime.root_node.is_some());
    let mutations = runtime.drain_mutations();
    assert!(!mutations.is_empty());

    // Check for specific mutations
    let has_create_element = mutations
        .iter()
        .any(|m| matches!(m, nexa_core::Mutation::CreateElement { .. }));
    let has_append = mutations
        .iter()
        .any(|m| matches!(m, nexa_core::Mutation::AppendChildren { .. }));

    assert!(has_create_element, "Should have CreateElement mutation");
    assert!(has_append, "Should have AppendChildren mutation");
}

#[test]
fn test_mount_text() {
    let mut runtime = create_test_runtime();

    fn root_component() -> NodeId {
        use nexa_core::vdom::get_active_arena;
        get_active_arena(|arena| {
            arena.insert(VirtualNode::Text(Text {
                text: "Hello World".to_string(),
                parent: None,
            }))
        })
    }

    runtime.mount("Root", root_component);

    let mutations = runtime.drain_mutations();
    let has_create_text = mutations.iter().any(
        |m| matches!(m, nexa_core::Mutation::CreateTextNode { text, .. } if text == "Hello World"),
    );
    assert!(
        has_create_text,
        "Should have CreateTextNode mutation with correct text"
    );
}

#[test]
fn test_attributes_and_listeners() {
    let mut runtime = create_test_runtime();

    fn root_component() -> NodeId {
        use nexa_core::vdom::get_active_arena;
        use smallvec::smallvec;

        get_active_arena(|arena| {
            arena.insert(VirtualNode::Element(Element {
                tag: "button",
                props: smallvec![Attribute {
                    name: "class",
                    value: "btn-primary".to_string()
                }],
                listeners: smallvec![EventListener {
                    name: "click",
                    cb: Rc::new(RefCell::new(|_| {}))
                }],
                children: Default::default(),
                parent: None,
                key: None,
            }))
        })
    }

    runtime.mount("Root", root_component);

    let mutations = runtime.drain_mutations();

    let has_attr = mutations.iter().any(|m| matches!(m, nexa_core::Mutation::SetAttribute { name, value, .. } if name == "class" && value == "btn-primary"));
    let has_listener = mutations.iter().any(
        |m| matches!(m, nexa_core::Mutation::NewEventListener { name, .. } if name == "click"),
    );

    assert!(has_attr, "Should have SetAttribute mutation");
    assert!(has_listener, "Should have NewEventListener mutation");
}

#[test]
fn test_mount_component() {
    let mut runtime = create_test_runtime();

    fn child_component() -> NodeId {
        use nexa_core::vdom::get_active_arena;
        get_active_arena(|arena| {
            arena.insert(VirtualNode::Element(Element {
                tag: "span",
                props: Default::default(),
                listeners: Default::default(),
                children: Default::default(),
                parent: None,
                key: None,
            }))
        })
    }

    fn root_component() -> NodeId {
        use nexa_core::vdom::get_active_arena;
        use smallvec::smallvec;

        // We need to insert the child component node first?
        // No, components are functions.
        // We need to insert a VirtualNode::Component that points to child_component.

        let child_node_id = get_active_arena(|arena| {
            arena.insert(VirtualNode::Component(nexa_core::Component {
                name: "Child",
                render_fn: child_component,
                scope: None,
                parent: None,
            }))
        });

        get_active_arena(|arena| {
            arena.insert(VirtualNode::Element(Element {
                tag: "div",
                props: Default::default(),
                listeners: Default::default(),
                children: smallvec![child_node_id],
                parent: None,
                key: None,
            }))
        })
    }

    runtime.mount("Root", root_component);

    let mutations = runtime.drain_mutations();

    // Expect: Create span, Create div, Append span to div, Append div to root.
    // Note: IDs are opaque. We just check structure.

    let create_span = mutations
        .iter()
        .any(|m| matches!(m, nexa_core::Mutation::CreateElement { tag, .. } if tag == "span"));
    let create_div = mutations
        .iter()
        .any(|m| matches!(m, nexa_core::Mutation::CreateElement { tag, .. } if tag == "div"));

    assert!(create_span, "Should create span from child component");
    assert!(create_div, "Should create div from root component");

    // Check append.
    // We expect AppendChildren { id: div_id, m: [span_id] }
    // It's hard to verify exact IDs without tracking them.
    // But existence of AppendChildren implies hierarchy was built.
    let append_count = mutations
        .iter()
        .filter(|m| matches!(m, nexa_core::Mutation::AppendChildren { .. }))
        .count();
    assert!(
        append_count >= 2,
        "Should have at least 2 appends (span->div, div->root)"
    );
}
