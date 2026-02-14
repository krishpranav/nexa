#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
}

pub struct LayoutContext {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum SceneNode {
    Rect(Rect),
    Text {
        x: f32,
        y: f32,
        content: String,
        font_size: f32,
        color: [f32; 4],
    },
    Image {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        src: String,
    },
    Container {
        transform: glam::Mat4,
        children: Vec<SceneNode>,
        is_dirty: bool,
    },
}

impl SceneNode {
    pub fn layout(&mut self, ctx: &LayoutContext) {
        // Simple layout logic: for now just pass down context
        if let SceneNode::Container { children, .. } = self {
            for child in children {
                child.layout(ctx);
            }
        }
    }

    pub fn mark_dirty(&mut self) {
        if let SceneNode::Container { is_dirty, .. } = self {
            *is_dirty = true;
        }
    }
}

pub struct Scene {
    pub root: SceneNode,
    pub last_frame_time: std::time::Duration,
}
