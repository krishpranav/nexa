use serde::{Deserialize, Serialize};

// API Surface for Nexa DevTools

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DevToolsMsg {
    Handshake { version: String },
    UpdateComponentTree(ComponentTree),
    UpdateSignalGraph(SignalGraphUpdate),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentTree {
    pub root: ComponentNode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentNode {
    pub id: u64,
    pub name: String,
    pub children: Vec<ComponentNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalGraphUpdate {
    pub signals: Vec<SignalInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalInfo {
    pub id: u64,
    pub value_preview: String,
    pub subscribers: Vec<u64>,
}

pub struct DevTools {
    // In a real impl, this would hold the WebSocket connection
    enabled: bool,
}

impl DevTools {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn connect(&self, url: &str) {
        if self.enabled {
            // Emulate connection logic
            tracing::info!("Connecting to DevTools at {}", url);
        }
    }

    pub fn send(&self, msg: DevToolsMsg) {
        if self.enabled {
            // Emulate send
            let json = serde_json::to_string(&msg).unwrap();
            tracing::debug!("DevTools Send: {}", json);
        }
    }
}
