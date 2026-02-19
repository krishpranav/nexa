use nexa_core::{Attribute, Element, NodeMetadata, Text, VirtualNode};
use nexa_rsx::rsx;

#[test]
fn test_element_expansion() {
    let mut arena = nexa_core::VDomArena::new();
    let nodes = unsafe {
        nexa_core::set_active_arena(&mut arena, || {
            rsx! {
                div { class: "foo" }
            }
        })
    };

    assert_eq!(nodes.len(), 1);
    let id = nodes[0];
    let node = arena.nodes.get(id).unwrap();

    if let VirtualNode::Element(el) = node {
        assert_eq!(el.tag, "div");
        assert_eq!(el.props.len(), 1);
        assert_eq!(el.props[0].name, "class");
        // Attribute value is String, so we compare with string literal
        assert_eq!(el.props[0].value, "foo");
    } else {
        panic!("Expected element");
    }
}

#[test]
fn test_component_expansion() {
    // Define a dummy component function
    fn MyComp(props: MyCompProps) -> nexa_core::NodeId {
        nexa_core::get_active_arena(|arena| {
            arena.insert(VirtualNode::Text(Text {
                text: format!("Value: {}", props.val),
                parent: None,
            }))
        })
    }

    struct MyCompProps {
        val: i32,
    }

    let mut arena = nexa_core::VDomArena::new();
    let nodes = unsafe {
        nexa_core::set_active_arena(&mut arena, || {
            rsx! {
                MyComp { val: 42 }
            }
        })
    };

    assert_eq!(nodes.len(), 1);
    // The node should be whatever MyComp returned.
    let id = nodes[0];
    let node = arena.nodes.get(id).unwrap();
    if let VirtualNode::Text(t) = node {
        assert_eq!(t.text, "Value: 42");
    } else {
        panic!("Expected text from component");
    }
}

#[test]
fn test_nested_structure() {
    let mut arena = nexa_core::VDomArena::new();
    let nodes = unsafe {
        nexa_core::set_active_arena(&mut arena, || {
            let val = 10;
            rsx! {
                div {
                    h1 { "Title" },
                    p { "Count: {val}" }
                }
            }
        })
    };

    // div -> h1, p
    let div_id = nodes[0];
    if let VirtualNode::Element(div) = arena.nodes.get(div_id).unwrap() {
        assert_eq!(div.children.len(), 2);
    } else {
        panic!("Expected div");
    }
}

#[test]
fn test_control_flow() {
    let mut arena = nexa_core::VDomArena::new();
    let nodes = unsafe {
        nexa_core::set_active_arena(&mut arena, || {
            let show = true;
            let items = vec!["a", "b"];
            rsx! {
                if show {
                    div { "Visible" }
                }
                for item in items {
                    span { "{item}" }
                }
            }
        })
    };

    // 1 div + 2 spans = 3 nodes
    assert_eq!(nodes.len(), 3);
}

#[test]
fn test_key_support() {
    let mut arena = nexa_core::VDomArena::new();
    let nodes = unsafe {
        nexa_core::set_active_arena(&mut arena, || {
            rsx! {
                div { key: "my-key" }
            }
        })
    };

    assert_eq!(nodes.len(), 1);
    let id = nodes[0];
    let node = arena.nodes.get(id).unwrap();

    if let VirtualNode::Element(el) = node {
        assert_eq!(el.key, Some("my-key".to_string()));
        // key should NOT be in props
        assert!(!el.props.iter().any(|p| p.name == "key"));
    } else {
        panic!("Expected element");
    }
}
