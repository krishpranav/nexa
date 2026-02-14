use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mutation {
    AppendChildren {
        id: u64,
        m: Vec<u64>,
    },
    AssignId {
        path: Vec<u8>,
        id: u64,
    },
    CreateElement {
        tag: String,
        id: u64,
    },
    CreatePlaceholder {
        id: u64,
    },
    CreateTextNode {
        text: String,
        id: u64,
    },
    HydrateText {
        path: Vec<u8>,
        value: String,
        id: u64,
    },
    LoadTemplate {
        name: String,
        index: usize,
        id: u64,
    },
    ReplaceWith {
        id: u64,
        m: Vec<u64>,
    },
    ReplacePlaceholder {
        path: Vec<u8>,
        m: Vec<u64>,
    },
    InsertAfter {
        id: u64,
        m: Vec<u64>,
    },
    InsertBefore {
        id: u64,
        m: Vec<u64>,
    },
    SetAttribute {
        name: String,
        value: String, // Value enum in real impl
        id: u64,
        ns: Option<String>,
    },
    SetText {
        value: String,
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
    Remove {
        id: u64,
    },
    PushRoot {
        id: u64,
    },
}
