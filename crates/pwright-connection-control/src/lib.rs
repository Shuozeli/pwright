//! Executor-neutral state machines for the Pwright graceful-close exchange.
//!
//! The machines perform no I/O and allocate no memory. Callers inspect the
//! next action, perform that side effect, and commit it at the side effect's
//! cancellation boundary. Atomic effects commit after success; a transport
//! that retains a partially sent frame commits immediately before its first
//! poll so resumption waits for the peer instead of submitting a duplicate.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::ControlMessage;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ClientClosePhase {
    #[default]
    Open,
    AwaitingCloseAck,
    ShutdownReady,
    Closed,
    Terminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientCloseAction {
    SendClose,
    AwaitCloseAck,
    ShutdownWrite,
    Complete,
    Stop,
}

impl ClientCloseAction {
    pub const fn message(self) -> Option<ControlMessage> {
        match self {
            Self::SendClose => Some(ControlMessage::Close),
            Self::AwaitCloseAck | Self::ShutdownWrite | Self::Complete | Self::Stop => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientCloseError {
    UnexpectedCompletion {
        phase: ClientClosePhase,
        action: ClientCloseAction,
    },
    UnexpectedMessage {
        phase: ClientClosePhase,
        message: ControlMessage,
    },
}

impl Display for ClientCloseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCompletion { phase, action } => write!(
                formatter,
                "cannot complete client close action {action:?} while in phase {phase:?}"
            ),
            Self::UnexpectedMessage { phase, message } => write!(
                formatter,
                "client close phase {phase:?} cannot receive control message {message:?}"
            ),
        }
    }
}

impl Error for ClientCloseError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClientCloseMachine {
    phase: ClientClosePhase,
}

impl ClientCloseMachine {
    pub const fn new() -> Self {
        Self {
            phase: ClientClosePhase::Open,
        }
    }

    pub const fn phase(self) -> ClientClosePhase {
        self.phase
    }

    pub const fn next_action(self) -> ClientCloseAction {
        match self.phase {
            ClientClosePhase::Open => ClientCloseAction::SendClose,
            ClientClosePhase::AwaitingCloseAck => ClientCloseAction::AwaitCloseAck,
            ClientClosePhase::ShutdownReady => ClientCloseAction::ShutdownWrite,
            ClientClosePhase::Closed => ClientCloseAction::Complete,
            ClientClosePhase::Terminated => ClientCloseAction::Stop,
        }
    }

    pub fn complete(
        &mut self,
        action: ClientCloseAction,
    ) -> Result<ClientClosePhase, ClientCloseError> {
        self.phase = match (self.phase, action) {
            (ClientClosePhase::Open, ClientCloseAction::SendClose) => {
                ClientClosePhase::AwaitingCloseAck
            }
            (ClientClosePhase::ShutdownReady, ClientCloseAction::ShutdownWrite) => {
                ClientClosePhase::Closed
            }
            (phase, action) => {
                return Err(ClientCloseError::UnexpectedCompletion { phase, action });
            }
        };
        Ok(self.phase)
    }

    pub fn receive(
        &mut self,
        message: ControlMessage,
    ) -> Result<ClientClosePhase, ClientCloseError> {
        if self.phase != ClientClosePhase::AwaitingCloseAck || message != ControlMessage::CloseAck {
            return Err(ClientCloseError::UnexpectedMessage {
                phase: self.phase,
                message,
            });
        }
        self.phase = ClientClosePhase::ShutdownReady;
        Ok(self.phase)
    }

    pub fn terminate(&mut self) -> ClientClosePhase {
        if self.phase != ClientClosePhase::Closed {
            self.phase = ClientClosePhase::Terminated;
        }
        self.phase
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ServerClosePhase {
    #[default]
    AwaitingClose,
    CleanupReady,
    CloseAckReady,
    ShutdownReady,
    Closed,
    Terminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerCloseAction {
    AwaitClose,
    Cleanup,
    SendCloseAck,
    ShutdownWrite,
    Complete,
    Stop,
}

impl ServerCloseAction {
    pub const fn message(self) -> Option<ControlMessage> {
        match self {
            Self::SendCloseAck => Some(ControlMessage::CloseAck),
            Self::AwaitClose
            | Self::Cleanup
            | Self::ShutdownWrite
            | Self::Complete
            | Self::Stop => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerCloseError {
    UnexpectedCompletion {
        phase: ServerClosePhase,
        action: ServerCloseAction,
    },
    UnexpectedMessage {
        phase: ServerClosePhase,
        message: ControlMessage,
    },
}

impl Display for ServerCloseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCompletion { phase, action } => write!(
                formatter,
                "cannot complete server close action {action:?} while in phase {phase:?}"
            ),
            Self::UnexpectedMessage { phase, message } => write!(
                formatter,
                "server close phase {phase:?} cannot receive control message {message:?}"
            ),
        }
    }
}

impl Error for ServerCloseError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ServerCloseMachine {
    phase: ServerClosePhase,
}

impl ServerCloseMachine {
    pub const fn new() -> Self {
        Self {
            phase: ServerClosePhase::AwaitingClose,
        }
    }

    pub const fn phase(self) -> ServerClosePhase {
        self.phase
    }

    pub const fn next_action(self) -> ServerCloseAction {
        match self.phase {
            ServerClosePhase::AwaitingClose => ServerCloseAction::AwaitClose,
            ServerClosePhase::CleanupReady => ServerCloseAction::Cleanup,
            ServerClosePhase::CloseAckReady => ServerCloseAction::SendCloseAck,
            ServerClosePhase::ShutdownReady => ServerCloseAction::ShutdownWrite,
            ServerClosePhase::Closed => ServerCloseAction::Complete,
            ServerClosePhase::Terminated => ServerCloseAction::Stop,
        }
    }

    pub fn receive(
        &mut self,
        message: ControlMessage,
    ) -> Result<ServerClosePhase, ServerCloseError> {
        if self.phase != ServerClosePhase::AwaitingClose || message != ControlMessage::Close {
            return Err(ServerCloseError::UnexpectedMessage {
                phase: self.phase,
                message,
            });
        }
        self.phase = ServerClosePhase::CleanupReady;
        Ok(self.phase)
    }

    pub fn complete(
        &mut self,
        action: ServerCloseAction,
    ) -> Result<ServerClosePhase, ServerCloseError> {
        self.phase = match (self.phase, action) {
            (ServerClosePhase::CleanupReady, ServerCloseAction::Cleanup) => {
                ServerClosePhase::CloseAckReady
            }
            (ServerClosePhase::CloseAckReady, ServerCloseAction::SendCloseAck) => {
                ServerClosePhase::ShutdownReady
            }
            (ServerClosePhase::ShutdownReady, ServerCloseAction::ShutdownWrite) => {
                ServerClosePhase::Closed
            }
            (phase, action) => {
                return Err(ServerCloseError::UnexpectedCompletion { phase, action });
            }
        };
        Ok(self.phase)
    }

    pub fn terminate(&mut self) -> ServerClosePhase {
        if self.phase != ServerClosePhase::Closed {
            self.phase = ServerClosePhase::Terminated;
        }
        self.phase
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::ControlMessage;

    use super::{
        ClientCloseAction, ClientCloseError, ClientCloseMachine, ClientClosePhase,
        ServerCloseAction, ServerCloseError, ServerCloseMachine, ServerClosePhase,
    };

    #[test]
    fn client_close_orders_message_ack_and_write_shutdown() {
        // Arrange
        let mut machine = ClientCloseMachine::new();

        // Act
        let initial = machine.next_action();
        let close_message = initial.message();
        machine.complete(initial).expect("close send should commit");
        let waiting = machine.next_action();
        machine
            .receive(ControlMessage::CloseAck)
            .expect("close ack should advance the barrier");
        let shutdown = machine.next_action();
        machine
            .complete(shutdown)
            .expect("write shutdown should commit");

        // Assert
        assert_eq!(initial, ClientCloseAction::SendClose);
        assert_eq!(close_message, Some(ControlMessage::Close));
        assert_eq!(waiting, ClientCloseAction::AwaitCloseAck);
        assert_eq!(shutdown, ClientCloseAction::ShutdownWrite);
        assert_eq!(machine.phase(), ClientClosePhase::Closed);
        assert_eq!(machine.next_action(), ClientCloseAction::Complete);
    }

    #[test]
    fn uncommitted_client_action_remains_retryable() {
        // Arrange
        let machine = ClientCloseMachine::new();

        // Act
        let first = machine.next_action();
        let retried = machine.next_action();

        // Assert
        assert_eq!(first, ClientCloseAction::SendClose);
        assert_eq!(retried, first);
        assert_eq!(machine.phase(), ClientClosePhase::Open);
    }

    #[test]
    fn client_rejects_ack_before_close_is_sent() {
        // Arrange
        let mut machine = ClientCloseMachine::new();

        // Act
        let error = machine
            .receive(ControlMessage::CloseAck)
            .expect_err("early close ack must fail");

        // Assert
        assert_eq!(
            error,
            ClientCloseError::UnexpectedMessage {
                phase: ClientClosePhase::Open,
                message: ControlMessage::CloseAck,
            }
        );
        assert_eq!(machine.phase(), ClientClosePhase::Open);
    }

    #[test]
    fn server_close_orders_cleanup_ack_and_write_shutdown() {
        // Arrange
        let mut machine = ServerCloseMachine::new();

        // Act
        machine
            .receive(ControlMessage::Close)
            .expect("close request should start server barrier");
        let cleanup = machine.next_action();
        machine.complete(cleanup).expect("cleanup should commit");
        let ack = machine.next_action();
        let ack_message = ack.message();
        machine.complete(ack).expect("ack send should commit");
        let shutdown = machine.next_action();
        machine
            .complete(shutdown)
            .expect("write shutdown should commit");

        // Assert
        assert_eq!(cleanup, ServerCloseAction::Cleanup);
        assert_eq!(ack, ServerCloseAction::SendCloseAck);
        assert_eq!(ack_message, Some(ControlMessage::CloseAck));
        assert_eq!(shutdown, ServerCloseAction::ShutdownWrite);
        assert_eq!(machine.phase(), ServerClosePhase::Closed);
        assert_eq!(machine.next_action(), ServerCloseAction::Complete);
    }

    #[test]
    fn server_rejects_ack_and_duplicate_close() {
        // Arrange
        let mut machine = ServerCloseMachine::new();
        let wrong_direction = machine
            .receive(ControlMessage::CloseAck)
            .expect_err("server must not accept close ack");
        machine
            .receive(ControlMessage::Close)
            .expect("first close should start the barrier");

        // Act
        let duplicate = machine
            .receive(ControlMessage::Close)
            .expect_err("duplicate close must fail");

        // Assert
        assert_eq!(
            wrong_direction,
            ServerCloseError::UnexpectedMessage {
                phase: ServerClosePhase::AwaitingClose,
                message: ControlMessage::CloseAck,
            }
        );
        assert_eq!(
            duplicate,
            ServerCloseError::UnexpectedMessage {
                phase: ServerClosePhase::CleanupReady,
                message: ControlMessage::Close,
            }
        );
    }

    #[test]
    fn termination_stops_incomplete_machines_but_preserves_completion() {
        // Arrange
        let mut client = ClientCloseMachine::new();
        let mut server = ServerCloseMachine::new();
        server
            .receive(ControlMessage::Close)
            .expect("server should accept close");

        // Act
        let client_phase = client.terminate();
        let server_phase = server.terminate();

        // Assert
        assert_eq!(client_phase, ClientClosePhase::Terminated);
        assert_eq!(server_phase, ServerClosePhase::Terminated);
        assert_eq!(client.next_action(), ClientCloseAction::Stop);
        assert_eq!(server.next_action(), ServerCloseAction::Stop);
    }
}
