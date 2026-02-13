use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// Simplified representation of the component tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNode {
    pub id: u64,
    pub name: String,
    pub children: Vec<u64>,
    pub props: serde_json::Value,
}

// Signal graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalNode {
    pub id: u64,
    pub value: String, // Stringified for display
    pub dependents: Vec<u64>,
    pub dependencies: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevToolsSnapshot {
    pub components: HashMap<u64, ComponentNode>,
    pub signals: HashMap<u64, SignalNode>,
    pub render_count: u64,
}

// Global DevTools context (singleton for simplicity in MVP)
// In a real app, this might be attached to the Runtime.
pub struct DevToolsContext {
    snapshot: Mutex<DevToolsSnapshot>,
}

impl DevToolsContext {
    pub fn new() -> Self {
        Self {
            snapshot: Mutex::new(DevToolsSnapshot::default()),
        }
    }

    pub fn update_component(&self, id: u64, name: String, children: Vec<u64>) {
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.components.insert(
            id,
            ComponentNode {
                id,
                name,
                children,
                props: serde_json::Value::Null,
            },
        );
    }

    pub fn update_signal(&self, id: u64, value: String) {
        let mut snapshot = self.snapshot.lock().unwrap();
        let node = snapshot.signals.entry(id).or_insert(SignalNode {
            id,
            value: String::new(),
            dependents: vec![],
            dependencies: vec![],
        });
        node.value = value;
    }

    pub fn increment_render_count(&self) {
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.render_count += 1;
    }

    pub fn get_snapshot_json(&self) -> String {
        let snapshot = self.snapshot.lock().unwrap();
        serde_json::to_string(&*snapshot).unwrap_or_default()
    }
}

// Static instance for easy access from anywhere (e.g. nexa-core)
// This is a "hook" point.
use std::sync::OnceLock;
static DEVTOOLS: OnceLock<DevToolsContext> = OnceLock::new();

pub fn devtools() -> &'static DevToolsContext {
    DEVTOOLS.get_or_init(DevToolsContext::new)
}
