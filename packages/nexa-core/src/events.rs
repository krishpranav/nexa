#[derive(Debug, Clone)]
pub enum Event {
    Click,
    Input(String),
    Unknown,
}
