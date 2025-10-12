use std::net::SocketAddr;

use korangar_gameplay::{GameplayEvent, SupportedPacketVersion};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) enum ServerConnectCommand {
    Login {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<GameplayEvent>,
        packet_version: SupportedPacketVersion,
    },
    Character {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<GameplayEvent>,
        packet_version: SupportedPacketVersion,
    },
    Map {
        address: SocketAddr,
        action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<GameplayEvent>,
        packet_version: SupportedPacketVersion,
    },
}

#[derive(Debug)]
pub(crate) enum NetworkTaskError {
    FailedToConnect,
    ConnectionClosed,
}

pub(crate) enum ServerConnection {
    Connected {
        action_sender: UnboundedSender<Vec<u8>>,
        event_receiver: UnboundedReceiver<GameplayEvent>,
        packet_version: SupportedPacketVersion,
    },
    ClosingManually,
    Disconnected,
}

impl ServerConnection {
    pub fn take(&mut self) -> Self {
        std::mem::replace(self, ServerConnection::Disconnected)
    }
}
