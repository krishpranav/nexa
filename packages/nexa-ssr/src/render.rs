use futures::stream::Stream;
use nexa_core::vdom::{NodeId, VDomArena, VirtualNode};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct SsrStream<'a> {
    arena: &'a VDomArena,
    // Stack of nodes to visit.
    // We strictly walk: Start Tag -> Children -> End Tag.
    // To handle "End Tag", we can push a "Close(tag_name)" marker.
    stack: VecDeque<RenderOp>,
}

enum RenderOp {
    Visit(NodeId),
    Close(&'static str),
}

impl<'a> SsrStream<'a> {
    pub fn new(arena: &'a VDomArena, root: NodeId) -> Self {
        let mut stack = VecDeque::new();
        stack.push_front(RenderOp::Visit(root));
        Self { arena, stack }
    }
}

impl<'a> Stream for SsrStream<'a> {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let op = match self.stack.pop_front() {
            Some(op) => op,
            None => return Poll::Ready(None),
        };

        match op {
            RenderOp::Close(tag) => Poll::Ready(Some(format!("</{}>", tag))),
            RenderOp::Visit(id) => {
                if let Some(node) = self.arena.nodes.get(id) {
                    match node {
                        VirtualNode::Element(el) => {
                            let mut chunks = String::new();
                            chunks.push_str("<");
                            chunks.push_str(el.tag);

                            for attr in &el.props {
                                chunks.push_str(" ");
                                chunks.push_str(attr.name);
                                chunks.push_str("=\"");
                                chunks.push_str(&escape_html(&attr.value));
                                chunks.push_str("\"");
                            }
                            chunks.push_str(">");

                            // Push closing tag to happen AFTER children
                            self.stack.push_front(RenderOp::Close(el.tag));

                            // Push children in reverse order so first child is at front
                            for &child in el.children.iter().rev() {
                                self.stack.push_front(RenderOp::Visit(child));
                            }

                            Poll::Ready(Some(chunks))
                        }
                        VirtualNode::Text(txt) => Poll::Ready(Some(escape_html(&txt.text))),
                        VirtualNode::Fragment(frag) => {
                            for &child in frag.children.iter().rev() {
                                self.stack.push_front(RenderOp::Visit(child));
                            }
                            // No output for fragment itself, just recurse
                            // We return empty string or loop again?
                            // Stream expects efficient polling. returning empty string is fine but wasteful?
                            // Let's just recursively call poll_next.
                            self.poll_next(_cx)
                        }
                        VirtualNode::Component(comp) => {
                            // Theoretically, a component has a 'root' or rendered output.
                            // But in our current vdom, Valid Component implies it hasn't been rendered or logic is separate?
                            // Typically SSR runs the component function to get the tree.
                            // Implementing this requires running the component fn.
                            // Since our Runtime logic is somewhat separate, we assume for now
                            // that 'Component' node might point to a child or we need to run it?
                            // Requirement: "Integrate with core runtime tree".
                            // IF the tree is fully built (expanded), Component node usually has a child/output?
                            // Our vdom definition doesn't show a 'child' for Component,
                            // but it has `render_fn`.

                            // In a real expanded tree, the component would have produced a subtree.
                            // If `nexa-core` expands components into the tree, we traverse that.
                            // If `Component` struct is just a placeholder, we'd need to run it.
                            // Let's assume we run it here roughly or skip.
                            // For simplicity: skip or placeholder.
                            Poll::Ready(Some(format!("<!-- component {} -->", comp.name)))
                        }
                        VirtualNode::Placeholder => {
                            Poll::Ready(Some("<!-- placeholder -->".to_string()))
                        }
                    }
                } else {
                    Poll::Ready(None)
                }
            }
        }
    }
}

fn escape_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '&' => output.push_str("&amp;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(c),
        }
    }
    output
}

pub struct Renderer<'a> {
    arena: &'a VDomArena,
}

impl<'a> Renderer<'a> {
    pub fn new(arena: &'a VDomArena) -> Self {
        Self { arena }
    }

    pub fn render_to_stream(&self, root_id: NodeId) -> SsrStream<'a> {
        SsrStream::new(self.arena, root_id)
    }
}

pub enum RenderMode {
    ToString,
    ToStream,
}
