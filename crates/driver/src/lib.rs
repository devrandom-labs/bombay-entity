//! Ordered concurrent execution for pure representable machines.
//!
//! A [`Driver`] owns the current value of a [`Machine`]. Inputs advance that
//! value under short synchronization and enqueue the corresponding outputs at
//! the same linearization point. Exactly one caller interprets queued outputs;
//! reentrant and concurrent callers append work for that interpreter.

#![deny(missing_docs)]

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Mutex;

pub use bombay_transition::Machine;

/// Interprets one machine output without driver synchronization held.
pub trait Handler<O> {
    /// Handle the next output in machine transition order.
    fn handle(&self, output: O);
}

impl<O, F> Handler<O> for F
where
    F: Fn(O),
{
    fn handle(&self, output: O) {
        self(output);
    }
}

/// A live, linearizable execution of one pure machine.
pub struct Driver<M, I>
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

impl<M, I> Driver<M, I>
where
    M: Machine<I>,
{
    /// Start driving `machine` with an empty output queue.
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
    /// evidence from the output. It must not call back into this driver.
    ///
    /// # Panics
    ///
    /// Panics after synchronization poison or if a previous machine transition
    /// panicked after taking ownership of the machine value.
    pub fn advance<T>(&self, input: I, observe: impl FnOnce(&M::Output, &M) -> T) -> T {
        let mut execution = self.execution.lock().expect("driver lock poisoned");
        let machine = execution.machine.take().expect("driver machine missing");
        let (output, successor) = machine.step(input);
        let observed = observe(&output, &successor);
        execution.machine = Some(successor);
        execution.outputs.push_back(output);
        observed
    }

    /// Inspect the current machine value under driver synchronization.
    ///
    /// The observer must be short and must not call back into this driver.
    ///
    /// # Panics
    ///
    /// Panics after synchronization poison or if a previous transition panic
    /// left the machine value unavailable.
    pub fn inspect<T>(&self, observe: impl FnOnce(&M) -> T) -> T {
        let execution = self.execution.lock().expect("driver lock poisoned");
        observe(execution.machine.as_ref().expect("driver machine missing"))
    }

    /// Interpret every currently or reentrantly queued output in transition order.
    ///
    /// Concurrent calls return when another caller already owns interpretation.
    /// The active caller continues until the ordered queue is empty.
    ///
    /// # Panics
    ///
    /// Panics if driver synchronization was poisoned.
    pub fn drive<H>(&self, handler: &H)
    where
        H: Handler<M::Output>,
    {
        {
            let mut execution = self.execution.lock().expect("driver lock poisoned");
            if execution.handling {
                return;
            }
            execution.handling = true;
        }
        while let Some(output) = self.next_output() {
            handler.handle(output);
        }
    }

    fn next_output(&self) -> Option<M::Output> {
        let mut execution = self.execution.lock().expect("driver lock poisoned");
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

    use super::Driver;

    const EMPTY: Topology = Topology {
        name: "sum",
        initial: bombay_transition::VertexId(0),
        vertices: &[],
        transitions: &[],
    };

    #[test]
    fn outputs_retain_transition_order() {
        let driver = Driver::new(Base::new(0_u8, EMPTY, |state: u8, input: u8| {
            (input, state.wrapping_add(input))
        }));
        driver.advance(1, |_, _| ());
        driver.advance(2, |_, _| ());
        driver.advance(3, |_, _| ());
        let outputs = Mutex::new(Vec::new());
        driver.drive(&|output| outputs.lock().unwrap().push(output));
        assert_eq!(*outputs.lock().unwrap(), [1, 2, 3]);
    }
}
