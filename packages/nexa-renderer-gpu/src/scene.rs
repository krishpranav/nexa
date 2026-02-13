#[derive(Debug, Clone)]
pub enum SceneNode {
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
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
        children: Vec<SceneNode>,
    },
}
