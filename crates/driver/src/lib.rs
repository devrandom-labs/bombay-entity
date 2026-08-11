//! Ordered concurrent execution for pure representable machines.
//!
//! A [`MachineExecutor`] owns the current value of a [`Machine`]. Inputs
//! advance that value under short synchronization and enqueue the corresponding
//! outputs at the same linearization point. Exactly one caller interprets
//! queued outputs; reentrant and concurrent callers append work for it.

#![deny(missing_docs)]

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Mutex;

pub use bombay_transition::Machine;

/// Interprets one machine output without executor synchronization held.
pub trait OutputInterpreter<O> {
    /// Handle the next output in machine transition order.
    fn handle(&self, output: O);
}

impl<O, F> OutputInterpreter<O> for F
where
    F: Fn(O),
{
    fn handle(&self, output: O) {
        self(output);
    }
}

/// A live, linearizable execution of one pure machine.
pub struct MachineExecutor<M, I>
where
    M: Machine<I>,
{
    execution: Mutex<Execution<M, M::Output>>,
    input: PhantomData<fn(I)>,
}

struct Execution<M, O> {
    machine: Option<M>,
    outputs: VecDeque<O>,
    handling: bool,
}

impl<M, I> MachineExecutor<M, I>
where
    M: Machine<I>,
{
    /// Create a live execution of `machine` with an empty output queue.
    #[must_use]
    pub fn new(machine: M) -> Self {
        Self {
            execution: Mutex::new(Execution {
                machine: Some(machine),
                outputs: VecDeque::new(),
                handling: false,
            }),
            input: PhantomData,
        }
    }

    /// Advance the machine and enqueue its output atomically.
    ///
    /// The observer runs while the transition is linearized and may copy small
    /// evidence from the output. It must not call back into this executor.
    ///
    /// # Panics
    ///
    /// Panics after synchronization poison or if a previous machine transition
    /// panicked after taking ownership of the machine value.
    pub fn submit<T>(&self, input: I, observe: impl FnOnce(&M::Output, &M) -> T) -> T {
        let mut execution = self.execution.lock().expect("executor lock poisoned");
        let machine = execution.machine.take().expect("executor machine missing");
        let (output, successor) = machine.step(input);
        let observed = observe(&output, &successor);
        execution.machine = Some(successor);
        execution.outputs.push_back(output);
        observed
    }

    /// Inspect the current machine under the same synchronization as advances.
    ///
    /// The observer must be short and must not call back into this executor.
    ///
    /// # Panics
    ///
    /// Panics after synchronization poison or if a transition panic consumed
    /// the current machine value.
    pub fn inspect<T>(&self, observe: impl FnOnce(&M) -> T) -> T {
        let execution = self.execution.lock().expect("executor lock poisoned");
        observe(
            execution
                .machine
                .as_ref()
                .expect("executor machine missing"),
        )
    }

    /// Interpret every currently or reentrantly queued output in transition order.
    ///
    /// Concurrent calls return when another caller already owns interpretation.
    /// The active caller continues until the ordered queue is empty.
    ///
    /// # Panics
    ///
    /// Panics if executor synchronization was poisoned.
    pub fn interpret_pending<H>(&self, interpreter: &H)
    where
        H: OutputInterpreter<M::Output>,
    {
        {
            let mut execution = self.execution.lock().expect("executor lock poisoned");
            if execution.handling {
                return;
            }
            execution.handling = true;
        }
        while let Some(output) = self.next_output() {
            interpreter.handle(output);
        }
    }

    fn next_output(&self) -> Option<M::Output> {
        let mut execution = self.execution.lock().expect("executor lock poisoned");
        if let Some(output) = execution.outputs.pop_front() {
            Some(output)
        } else {
            execution.handling = false;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use bombay_transition::{Base, Topology};

    use super::MachineExecutor;

    const EMPTY: Topology = Topology {
        name: "sum",
        initial: bombay_transition::VertexId(0),
        vertices: &[],
        transitions: &[],
    };

    #[test]
    fn outputs_retain_transition_order() {
        let executor = MachineExecutor::new(Base::new(0_u8, EMPTY, |state: u8, input: u8| {
            (input, state.wrapping_add(input))
        }));
        executor.submit(1, |_, _| ());
        executor.submit(2, |_, _| ());
        executor.submit(3, |_, _| ());
        let outputs = Mutex::new(Vec::new());
        executor.interpret_pending(&|output| outputs.lock().unwrap().push(output));
        assert_eq!(*outputs.lock().unwrap(), [1, 2, 3]);
    }
}
