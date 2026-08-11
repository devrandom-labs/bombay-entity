//! Pure entity-specific behavior protocol composition.

use behavior::{
    Actions, Behavior, ChildEvent, ChildStopped, CreationEvent, CreationResolved, Delivery,
    PeerEvent, PeerStopped, Recipient, SendAlgebra, SendProduct, ShutdownEvent, ShutdownRequested,
    TimeEvent, TimerElapsed, User, UserEvent, WorkerCreationEvent, WorkerCreationResolved,
    WorkerEvent, WorkerStopped,
};

/// Confirmation that an entity drain fence reached its behavior transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainFenceAcknowledged;

/// Commands accepted by an entity-wrapped behavior.
#[derive(Clone, PartialEq, Eq)]
pub enum EntityProtocol<A: behavior::Address, C> {
    /// An application command forwarded unchanged to the inner behavior.
    Command(C),
    /// An ordered processing fence acknowledged after preceding transitions.
    DrainFence {
        /// Typed destination for the fence acknowledgement.
        reply_to: Recipient<A, DrainFenceAcknowledged>,
    },
}

/// Complete event protocol for an entity-wrapped behavior.
#[derive(Clone, PartialEq, Eq)]
pub enum EntityEvent<E: UserEvent> {
    /// An event belonging to the wrapped behavior.
    Inner(E),
    /// An entity drain fence with its original sender.
    DrainFence {
        /// Actor address that submitted the fence.
        from: E::Addr,
        /// Typed destination for acknowledgement.
        reply_to: Recipient<E::Addr, DrainFenceAcknowledged>,
    },
}

impl<E: UserEvent> UserEvent for EntityEvent<E> {
    type Addr = E::Addr;
    type Message = EntityProtocol<E::Addr, E::Message>;

    fn user(from: Self::Addr, message: Self::Message) -> Self {
        match message {
            EntityProtocol::Command(command) => Self::Inner(E::user(from, command)),
            EntityProtocol::DrainFence { reply_to } => Self::DrainFence { from, reply_to },
        }
    }

    fn into_user(self) -> Result<User<Self::Addr, Self::Message>, Self> {
        match self {
            Self::Inner(event) => event
                .into_user()
                .map(|user| User::new(user.from, EntityProtocol::Command(user.message)))
                .map_err(Self::Inner),
            Self::DrainFence { from, reply_to } => {
                Ok(User::new(from, EntityProtocol::DrainFence { reply_to }))
            }
        }
    }
}

macro_rules! forward_optional_event {
    ($trait:ident, $method:ident, $event:ty) => {
        impl<E> $trait for EntityEvent<E>
        where
            E: UserEvent + $trait,
        {
            fn $method(event: $event) -> Option<Self> {
                E::$method(event).map(Self::Inner)
            }
        }
    };
}

forward_optional_event!(TimeEvent, time_reached, TimerElapsed);
forward_optional_event!(PeerEvent, peer_stopped, PeerStopped<E::Addr>);
forward_optional_event!(ChildEvent, child_stopped, ChildStopped<E::Addr>);
forward_optional_event!(WorkerEvent, worker_stopped, WorkerStopped<E::Addr>);
forward_optional_event!(
    CreationEvent,
    creation_resolved,
    CreationResolved<<E::Addr as behavior::Address>::Nonce>
);
forward_optional_event!(
    WorkerCreationEvent,
    worker_creation_resolved,
    WorkerCreationResolved<<E::Addr as behavior::Address>::Nonce>
);
forward_optional_event!(ShutdownEvent, shutdown_requested, ShutdownRequested);

/// Entity-runtime protocol composition around an application behavior.
pub struct EntityBehavior<B> {
    inner: B,
}

impl<B> EntityBehavior<B> {
    /// Wrap an application behavior with entity command and drain protocols.
    pub const fn new(inner: B) -> Self {
        Self { inner }
    }

    /// Borrow the wrapped behavior.
    pub const fn inner(&self) -> &B {
        &self.inner
    }

    /// Return the wrapped behavior.
    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: Behavior> Behavior for EntityBehavior<B> {
    type Addr = B::Addr;
    type Msg = EntityProtocol<B::Addr, B::Msg>;
    type Event = EntityEvent<B::Event>;
    type Sends = SendProduct<B::Sends, Vec<Delivery<B::Addr, DrainFenceAcknowledged>>>;
    type Ph = B::Ph;
    type Error = B::Error;
    type Birth = B::Birth;

    fn init(&mut self) -> behavior::BehaviorActed<Self> {
        self.inner
            .init()
            .map(|actions| actions.map_sends(|sends| SendProduct::new(sends, Vec::new())))
    }

    fn transition(&mut self, event: Self::Event) -> behavior::BehaviorActed<Self> {
        match event {
            EntityEvent::Inner(event) => self
                .inner
                .transition(event)
                .map(|actions| actions.map_sends(|sends| SendProduct::new(sends, Vec::new()))),
            EntityEvent::DrainFence { reply_to, .. } => Ok(Actions::new(
                SendProduct::new(
                    B::Sends::empty(),
                    vec![Delivery::new(reply_to, DrainFenceAcknowledged)],
                ),
                Vec::new(),
                behavior::Step::Continue,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use behavior::{Actions, Handler, MailAddr, Never, NoBirths, Pure, Route};

    use super::{DrainFenceAcknowledged, EntityBehavior, EntityEvent, EntityProtocol};
    use behavior::{Behavior, Recipient, UserEvent};

    struct Counter(usize);

    impl Handler for Counter {
        type Addr = MailAddr;
        type Msg = u8;

        fn receive(
            &mut self,
            _from: MailAddr,
            _message: u8,
        ) -> behavior::Acted<
            MailAddr,
            Never,
            Vec<behavior::Delivery<MailAddr, Never>>,
            NoBirths,
            Never,
        > {
            self.0 += 1;
            Ok(Actions::cont())
        }
    }

    #[test]
    fn command_is_forwarded_unchanged_to_inner_behavior() {
        let mut behavior = EntityBehavior::new(Pure::new(Counter(0)));
        let event = UserEvent::user(MailAddr(1), EntityProtocol::Command(7));

        let actions = behavior.transition(event).unwrap();

        assert_eq!(behavior.inner().state().0, 1);
        assert!(actions.sends.inner.is_empty());
        assert!(actions.sends.own.is_empty());
    }

    #[test]
    fn drain_fence_preserves_inner_state_and_emits_only_acknowledgement() {
        let mut behavior = EntityBehavior::new(Pure::new(Counter(0)));
        let reply_to = Recipient::global(MailAddr(9));
        let event = EntityEvent::user(MailAddr(1), EntityProtocol::DrainFence { reply_to });

        let actions = behavior.transition(event).unwrap();

        assert_eq!(behavior.inner().state().0, 0);
        assert!(actions.sends.inner.is_empty());
        assert_eq!(actions.sends.own.len(), 1);
        assert_eq!(actions.sends.own[0].to.route(), Route::Global(MailAddr(9)));
        assert_eq!(actions.sends.own[0].message, DrainFenceAcknowledged);
        assert!(actions.creates.is_empty());
        assert!(matches!(actions.become_, behavior::Step::Continue));
    }
}
