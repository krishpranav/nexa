use futures::stream::Stream;
use nexa_core::vdom::{NodeId, VDomArena};

pub enum RenderMode {
    Client,
    Server,
    Hydrate,
}

pub struct Renderer<'a> {
    _arena: &'a VDomArena,
    _mode: RenderMode,
}

impl<'a> Renderer<'a> {
    pub fn new(arena: &'a VDomArena) -> Self {
        Self {
            _arena: arena,
            _mode: RenderMode::Server,
        }
    }

    pub fn render_to_stream(&self, _root_id: NodeId) -> impl Stream<Item = String> + '_ {
        // Minimal streaming implementation stub.
        // In reality, this would be a complex state machine iterating the VDOM.
        futures::stream::iter(vec![
            "<html>".to_string(),
            "<body>".to_string(),
            // Actual traversal would happen here yielding results
            "</body>".to_string(),
            "</html>".to_string(),
        ])
    }
}
