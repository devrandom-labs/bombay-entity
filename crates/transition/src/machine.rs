//! Statically composed, structurally representable executable machines.

/// One declared edge in a base machine's topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// Source vertex label.
    pub from: &'static str,
    /// Input label selecting the edge.
    pub input: &'static str,
    /// Destination vertex label.
    pub to: &'static str,
}

/// Inspectable metadata for one indivisible machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Topology {
    /// Stable component name.
    pub name: &'static str,
    /// Declared transition graph.
    pub transitions: &'static [Transition],
}

/// A stateful transducer whose composition remains structurally inspectable.
pub trait Machine<I> {
    /// Output produced for one input.
    type Output;

    /// Consume one input and return the output plus successor machine.
    fn step(self, input: I) -> (Self::Output, Self)
    where
        Self: Sized;

    /// Fold the retained composition tree with a structural interpreter.
    fn describe<V: Structure>(&self, visitor: &mut V) -> V::Output;
}

/// Structural composition operations available to every machine.
pub trait Compose<I>: Machine<I> + Sized {
    /// Compose this machine sequentially with another machine.
    fn then<N>(self, next: N) -> Then<Self, N> {
        Then(self, next)
    }

    /// Compose this machine in a product with another machine.
    fn product<N>(self, other: N) -> Product<Self, N> {
        Product(self, other)
    }

    /// Compose this machine as an alternative to another machine.
    fn choice<N>(self, other: N) -> Choice<Self, N> {
        Choice(self, other)
    }
}

impl<I, M: Machine<I>> Compose<I> for M {}

/// Interpreter for the composition structure of a machine.
pub trait Structure {
    /// Representation produced for each subtree.
    type Output;

    /// Interpret an indivisible machine.
    fn base(&mut self, topology: Topology) -> Self::Output;

    /// Interpret sequential composition.
    fn then(&mut self, first: Self::Output, second: Self::Output) -> Self::Output;

    /// Interpret product composition.
    fn product(&mut self, left: Self::Output, right: Self::Output) -> Self::Output;

    /// Interpret sum composition.
    fn choice(&mut self, left: Self::Output, right: Self::Output) -> Self::Output;
}

/// An indivisible stateful machine and its inspectable topology.
pub struct Base<S, F> {
    state: S,
    transition: F,
    topology: Topology,
}

impl<S, F> Base<S, F> {
    /// Construct a base machine from state, topology, and transition function.
    pub const fn new(state: S, topology: Topology, transition: F) -> Self {
        Self {
            state,
            transition,
            topology,
        }
    }
}

impl<S, I, O, F> Machine<I> for Base<S, F>
where
    F: FnMut(S, I) -> (O, S),
{
    type Output = O;

    fn step(self, input: I) -> (Self::Output, Self) {
        let Self {
            state,
            mut transition,
            topology,
        } = self;
        let (output, state) = transition(state, input);
        (
            output,
            Self {
                state,
                transition,
                topology,
            },
        )
    }

    fn describe<V: Structure>(&self, visitor: &mut V) -> V::Output {
        visitor.base(self.topology)
    }
}

/// Sequential composition of two machines.
pub struct Then<A, B>(A, B);

impl<I, A, B> Machine<I> for Then<A, B>
where
    A: Machine<I>,
    B: Machine<A::Output>,
{
    type Output = B::Output;

    fn step(self, input: I) -> (Self::Output, Self) {
        let (middle, first) = self.0.step(input);
        let (output, second) = self.1.step(middle);
        (output, Self(first, second))
    }

    fn describe<V: Structure>(&self, visitor: &mut V) -> V::Output {
        let first = self.0.describe(visitor);
        let second = self.1.describe(visitor);
        visitor.then(first, second)
    }
}

/// Product composition of two independent machines.
pub struct Product<A, B>(A, B);

impl<I, J, A, B> Machine<(I, J)> for Product<A, B>
where
    A: Machine<I>,
    B: Machine<J>,
{
    type Output = (A::Output, B::Output);

    fn step(self, (left, right): (I, J)) -> (Self::Output, Self) {
        let (left_output, left_machine) = self.0.step(left);
        let (right_output, right_machine) = self.1.step(right);
        (
            (left_output, right_output),
            Self(left_machine, right_machine),
        )
    }

    fn describe<V: Structure>(&self, visitor: &mut V) -> V::Output {
        let left = self.0.describe(visitor);
        let right = self.1.describe(visitor);
        visitor.product(left, right)
    }
}

/// One of two alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Either<L, R> {
    /// Left alternative.
    Left(L),
    /// Right alternative.
    Right(R),
}

/// Sum composition routing each alternative to its corresponding machine.
pub struct Choice<A, B>(A, B);

impl<I, J, A, B> Machine<Either<I, J>> for Choice<A, B>
where
    A: Machine<I>,
    B: Machine<J>,
{
    type Output = Either<A::Output, B::Output>;

    fn step(self, input: Either<I, J>) -> (Self::Output, Self) {
        match input {
            Either::Left(left) => {
                let (output, machine) = self.0.step(left);
                (Either::Left(output), Self(machine, self.1))
            }
            Either::Right(right) => {
                let (output, machine) = self.1.step(right);
                (Either::Right(output), Self(self.0, machine))
            }
        }
    }

    fn describe<V: Structure>(&self, visitor: &mut V) -> V::Output {
        let left = self.0.describe(visitor);
        let right = self.1.describe(visitor);
        visitor.choice(left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::{Base, Compose, Machine, Structure, Topology, Transition};

    const EDGES: &[Transition] = &[Transition {
        from: "ready",
        input: "advance",
        to: "ready",
    }];

    fn topology(name: &'static str) -> Topology {
        Topology {
            name,
            transitions: EDGES,
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Shape {
        bases: usize,
        sequences: usize,
        products: usize,
        choices: usize,
    }

    struct Count;

    impl Structure for Count {
        type Output = Shape;

        fn base(&mut self, _topology: Topology) -> Self::Output {
            Shape {
                bases: 1,
                sequences: 0,
                products: 0,
                choices: 0,
            }
        }

        fn then(&mut self, first: Self::Output, second: Self::Output) -> Self::Output {
            Shape {
                bases: first.bases + second.bases,
                sequences: first.sequences + second.sequences + 1,
                products: first.products + second.products,
                choices: first.choices + second.choices,
            }
        }

        fn product(&mut self, left: Self::Output, right: Self::Output) -> Self::Output {
            Shape {
                bases: left.bases + right.bases,
                sequences: left.sequences + right.sequences,
                products: left.products + right.products + 1,
                choices: left.choices + right.choices,
            }
        }

        fn choice(&mut self, left: Self::Output, right: Self::Output) -> Self::Output {
            Shape {
                bases: left.bases + right.bases,
                sequences: left.sequences + right.sequences,
                products: left.products + right.products,
                choices: left.choices + right.choices + 1,
            }
        }
    }

    #[test]
    fn sequential_machine_executes_and_retains_its_structure() {
        let increment = Base::new(0_u8, topology("increment"), |state: u8, input: u8| {
            let state = state + input;
            (state, state)
        });
        let double = Base::new((), topology("double"), |(), input: u8| (input * 2, ()));
        let machine = increment.then(double);

        let (output, machine) = machine.step(3);
        assert_eq!(output, 6);
        let (output, machine) = machine.step(1);
        assert_eq!(output, 8);
        assert_eq!(
            machine.describe(&mut Count),
            Shape {
                bases: 2,
                sequences: 1,
                products: 0,
                choices: 0,
            }
        );
    }
}
