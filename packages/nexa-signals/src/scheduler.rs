use crate::{Graph, SignalId};

/// A trait for scheduling updates in the reactive system.
/// This allows the scheduling logic to be decoupled from the core runtime.
pub trait Scheduler {
    /// Add signals to the set of dirty signals to be processed.
    fn schedule(&mut self, dirty: impl IntoIterator<Item = SignalId>);

    /// Run the scheduler to determine the execution order of effects.
    /// Returns a list of SignalIds sorted by execution order.
    fn run(&mut self, graph: &Graph) -> Vec<SignalId>;
}
