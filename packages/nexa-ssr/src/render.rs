use futures::stream::Stream;
use nexa_core::vdom::{NodeId, VDomArena, VirtualNode};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, Copy)]
pub struct SsrConfig {
    pub chunk_size: usize,
    pub enable_hydration: bool,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            chunk_size: 4096,
            enable_hydration: true,
        }
    }
}

pub struct SsrStream<'a> {
    arena: &'a VDomArena,
    config: SsrConfig,
    stack: VecDeque<RenderOp>,
    buffer: String,
    suspense_tasks: VecDeque<SuspenseTask>,
    next_suspense_id: u32,
}

enum RenderOp {
    Visit(NodeId),
    Close(&'static str),
}

struct SuspenseTask {
    id: u32,
    actual_id: NodeId,
}

impl<'a> SsrStream<'a> {
    pub fn new(arena: &'a VDomArena, root: NodeId, config: SsrConfig) -> Self {
        let mut stack = VecDeque::new();
        stack.push_front(RenderOp::Visit(root));
        Self {
            arena,
            config,
            stack,
            buffer: String::with_capacity(config.chunk_size),
            suspense_tasks: VecDeque::new(),
            next_suspense_id: 0,
        }
    }

    fn render_node(&mut self, id: NodeId) -> Option<String> {
        let node = self.arena.nodes.get(id)?;
        match node {
            VirtualNode::Element(el) => {
                let mut out = format!("<{}", el.tag);

                // Hydration marker
                if self.config.enable_hydration {
                    // We use the raw SlotMap index as a stable ID for hydration
                    // For now, representing it simply.
                    out.push_str(&format!(" data-nexa-id=\"{:?}\"", id));
                }

                for attr in &el.props {
                    if is_boolean_attribute(attr.name) {
                        if attr.value == "true" {
                            out.push_str(&format!(" {}", attr.name));
                        }
                    } else {
                        out.push_str(&format!(" {}=\"{}\"", attr.name, escape_html(&attr.value)));
                    }
                }
                out.push('>');

                self.stack.push_front(RenderOp::Close(el.tag));
                for &child in el.children.iter().rev() {
                    self.stack.push_front(RenderOp::Visit(child));
                }
                Some(out)
            }
            VirtualNode::Text(txt) => Some(escape_html(&txt.text)),
            VirtualNode::Fragment(frag) => {
                for &child in frag.children.iter().rev() {
                    self.stack.push_front(RenderOp::Visit(child));
                }
                None
            }
            VirtualNode::Suspense(s) => {
                let s_id = self.next_suspense_id;
                self.next_suspense_id += 1;

                // Emit placeholder
                let out = format!("<div id=\"suspense-fallback-{}\">", s_id);
                self.stack.push_front(RenderOp::Close("div"));
                self.stack.push_front(RenderOp::Visit(s.fallback));

                self.suspense_tasks.push_back(SuspenseTask {
                    id: s_id,
                    actual_id: s.actual,
                });

                Some(out)
            }
            VirtualNode::Component(comp) => {
                // In a production SSR, we'd have expanded components.
                // If not, we emit a comment.
                Some(format!("<!-- component: {} -->", comp.name))
            }
            VirtualNode::Placeholder => Some("<!-- nexa-placeholder -->".to_string()),
        }
    }
}

impl<'a> Stream for SsrStream<'a> {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        while self.buffer.len() < self.config.chunk_size {
            if let Some(op) = self.stack.pop_front() {
                match op {
                    RenderOp::Visit(id) => {
                        if let Some(chunk) = self.render_node(id) {
                            self.buffer.push_str(&chunk);
                        }
                    }
                    RenderOp::Close(tag) => {
                        self.buffer.push_str("</");
                        self.buffer.push_str(tag);
                        self.buffer.push_str(">");
                    }
                }
            } else if let Some(task) = self.suspense_tasks.pop_front() {
                // For a real async suspense, we'd wait for a future.
                // For this implementation, we emit the "patch" immediately in the same stream
                // to demonstrate the mechanism.

                let mut sub_stream = SsrStream::new(
                    self.arena,
                    task.actual_id,
                    SsrConfig {
                        chunk_size: 1000000, // No chunking for subtrees
                        enable_hydration: self.config.enable_hydration,
                    },
                );

                let mut content = String::new();
                while let Some(chunk) = sub_stream.stack.pop_front() {
                    match chunk {
                        RenderOp::Visit(id) => {
                            if let Some(c) = sub_stream.render_node(id) {
                                content.push_str(&c);
                            }
                        }
                        RenderOp::Close(tag) => {
                            content.push_str("</");
                            content.push_str(tag);
                            content.push_str(">");
                        }
                    }
                }

                // Patching script
                let patch = format!(
                    "<template id=\"suspense-content-{}\">{}</template>\
                    <script>\
                        (function() {{\
                            var fallback = document.getElementById('suspense-fallback-{}');\
                            var content = document.getElementById('suspense-content-{}').content;\
                            fallback.parentNode.replaceChild(content, fallback);\
                        }})();\
                    </script>",
                    task.id, content, task.id, task.id
                );
                self.buffer.push_str(&patch);
            } else {
                break;
            }
        }

        if !self.buffer.is_empty() {
            let out = std::mem::take(&mut self.buffer);
            Poll::Ready(Some(out))
        } else {
            Poll::Ready(None)
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
            '\'' => output.push_str("&#39;"),
            _ => output.push(c),
        }
    }
    output
}

fn is_boolean_attribute(name: &str) -> bool {
    matches!(
        name,
        "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "ismap"
            | "loop"
            | "multiple"
            | "muted"
            | "nomodule"
            | "novalidate"
            | "open"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
            | "typemustmatch"
    )
}

pub struct Renderer<'a> {
    arena: &'a VDomArena,
    config: SsrConfig,
}

impl<'a> Renderer<'a> {
    pub fn new(arena: &'a VDomArena) -> Self {
        Self {
            arena,
            config: SsrConfig::default(),
        }
    }

    pub fn with_config(arena: &'a VDomArena, config: SsrConfig) -> Self {
        Self { arena, config }
    }

    pub fn render_to_stream(&self, root_id: NodeId) -> SsrStream<'a> {
        SsrStream::new(
            self.arena,
            root_id,
            SsrConfig {
                chunk_size: self.config.chunk_size,
                enable_hydration: self.config.enable_hydration,
            },
        )
    }
}
