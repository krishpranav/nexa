#[cfg(debug_assertions)]
mod internal {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ComponentNode {
        pub id: u64,
        pub name: String,
        pub children: Vec<u64>,
        pub props: serde_json::Value,
        pub location: Option<String>, // e.g., "src/main.rs:12"
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SignalNode {
        pub id: u64,
        pub label: String,
        pub value: String,
        pub dependents: Vec<u64>,
        pub dependencies: Vec<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct SchedulerMetrics {
        pub pending_tasks: usize,
        pub total_tasks_executed: u64,
        pub average_latency_ms: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct DevToolsSnapshot {
        pub components: HashMap<u64, ComponentNode>,
        pub signals: HashMap<u64, SignalNode>,
        pub render_count: u64,
        pub metrics: SchedulerMetrics,
        pub timestamp: u64,
    }

    pub struct DevToolsContext {
        snapshot: Mutex<DevToolsSnapshot>,
        bridge: Mutex<Option<Box<dyn DevBridge>>>,
    }

    pub trait DevBridge: Send + Sync {
        fn send_snapshot(&self, snapshot: &DevToolsSnapshot);
        fn on_command(&self, cmd: String);
    }

    impl DevToolsContext {
        pub fn new() -> Self {
            Self {
                snapshot: Mutex::new(DevToolsSnapshot::default()),
                bridge: Mutex::new(None),
            }
        }

        pub fn set_bridge(&self, bridge: Box<dyn DevBridge>) {
            let mut b = self.bridge.lock().unwrap();
            *b = Some(bridge);
        }

        pub fn update_component(
            &self,
            id: u64,
            name: String,
            children: Vec<u64>,
            location: Option<String>,
        ) {
            let mut snapshot = self.snapshot.lock().unwrap();
            snapshot.components.insert(
                id,
                ComponentNode {
                    id,
                    name,
                    children,
                    props: serde_json::Value::Null,
                    location,
                },
            );
        }

        pub fn update_signal(&self, id: u64, label: String, value: String, deps: Vec<u64>) {
            let mut snapshot = self.snapshot.lock().unwrap();
            let node = snapshot.signals.entry(id).or_insert(SignalNode {
                id,
                label: label.clone(),
                value: String::new(),
                dependents: vec![],
                dependencies: deps,
            });
            node.label = label;
            node.value = value;
        }

        pub fn record_render(&self) {
            let mut snapshot = self.snapshot.lock().unwrap();
            snapshot.render_count += 1;

            // Auto-push to bridge if exists
            if let Some(bridge) = self.bridge.lock().unwrap().as_ref() {
                bridge.send_snapshot(&snapshot);
            }
        }

        pub fn update_metrics(&self, pending: usize, total: u64, latency: f64) {
            let mut snapshot = self.snapshot.lock().unwrap();
            snapshot.metrics = SchedulerMetrics {
                pending_tasks: pending,
                total_tasks_executed: total,
                average_latency_ms: latency,
            };
        }

        pub fn export_state(&self) -> String {
            let snapshot = self.snapshot.lock().unwrap();
            serde_json::to_string(&*snapshot).unwrap_or_default()
        }
    }

    use std::sync::OnceLock;
    pub static DEVTOOLS: OnceLock<DevToolsContext> = OnceLock::new();
}

#[cfg(debug_assertions)]
pub use internal::*;

// Production no-op implementation
#[cfg(not(debug_assertions))]
pub mod production {
    pub struct DevToolsContext;
    impl DevToolsContext {
        pub fn update_component(&self, _: u64, _: String, _: Vec<u64>, _: Option<String>) {}
        pub fn update_signal(&self, _: u64, _: String, _: String, _: Vec<u64>) {}
        pub fn record_render(&self) {}
        pub fn update_metrics(&self, _: usize, _: u64, _: f64) {}
    }
    pub static DEVTOOLS: DevToolsContext = DevToolsContext;
}

#[cfg(not(debug_assertions))]
pub use production::*;

pub fn devtools() -> &'static DevToolsContext {
    #[cfg(debug_assertions)]
    {
        DEVTOOLS.get_or_init(DevToolsContext::new)
    }
    #[cfg(not(debug_assertions))]
    {
        &DEVTOOLS
    }
}
