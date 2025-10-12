use std::net::IpAddr;

use ragnarok_packets::{AccountId, CharacterId, Sex};

/// Data required for login server authentication.
#[derive(Debug, Clone, Copy)]
pub struct LoginServerLoginData {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    pub sex: Sex,
}

/// Unified login failure reasons across different packet versions.
#[derive(Debug, Clone, Copy)]
pub enum UnifiedLoginFailedReason {
    ServerClosed,
    AlreadyLoggedIn,
    AlreadyOnline,
    UnregisteredId,
    IncorrectPassword,
    IdExpired,
    RejectedFromServer,
    BlockedByGMTeam,
    GameOutdated,
    LoginProhibitedUntil,
    ServerFull,
    CompanyAccountLimitReached,
}

/// Unified character selection failure reasons.
#[derive(Debug, Clone, Copy)]
pub enum UnifiedCharacterSelectionFailedReason {
    RejectedFromServer,
    MapServerUnavailable,
}

/// Data received after successful character selection.
#[derive(Debug, Clone, Copy)]
pub struct CharacterServerLoginData {
    pub server_ip: IpAddr,
    pub server_port: u16,
    pub character_id: CharacterId,
}

/// Error indicating that an operation was attempted without being connected.
#[derive(Debug)]
pub struct NotConnectedError;

/// Result type for gameplay operations.
pub type GameplayResult<T = ()> = Result<T, GameplayError>;

/// Errors that can occur during gameplay operations.
#[derive(Debug)]
pub enum GameplayError {
    NotConnected,
    InvalidData,
    OperationFailed,
}

impl From<NotConnectedError> for GameplayError {
    fn from(_: NotConnectedError) -> Self {
        GameplayError::NotConnected
    }
}
