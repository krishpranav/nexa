use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mutation {
    AppendChildren {
        id: u64,
        m: usize,
    },
    AssignId {
        path: Vec<u8>,
        id: u64,
    },
    CreatePlaceholder {
        id: u64,
    },
    CreateTextNode {
        value: String,
        id: u64,
    },
    CreateElement {
        name: String,
        id: u64,
    },
    NewEventListener {
        name: String,
        id: u64,
    },
    RemoveEventListener {
        name: String,
        id: u64,
    },
    SetText {
        value: String,
        id: u64,
    },
    SetAttribute {
        name: String,
        value: String, // Simplified for now, could be dynamic (JSON/enum)
        id: u64,
        ns: Option<String>,
    },
    RemoveAttribute {
        name: String,
        id: u64,
        ns: Option<String>,
    },
    InsertAfter {
        id: u64,
        m: usize,
    },
    InsertBefore {
        id: u64,
        m: usize,
    },
    ReplaceWith {
        id: u64,
        m: usize,
    },
    ReplacePlaceholder {
        path: Vec<u8>,
        m: usize,
    },
    Remove {
        id: u64,
    },
    PushRoot {
        id: u64,
    },
}
