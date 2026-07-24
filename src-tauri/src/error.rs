use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("serial device not found: {0}")]
    DeviceNotFound(String),
    #[error("serial device is busy")]
    DeviceBusy,
    #[error("no recording is available")]
    NoRecording,
    #[error("invalid recording: {0}")]
    InvalidRecording(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("timeout while {0}")]
    Timeout(String),
    #[error("serial error: {0}")]
    Serial(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

impl From<serialport::Error> for CoreError {
    fn from(value: serialport::Error) -> Self {
        Self::Serial(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        let code = match error {
            CoreError::DeviceNotFound(_) => "DEVICE_NOT_FOUND",
            CoreError::DeviceBusy => "DEVICE_BUSY",
            CoreError::NoRecording => "NO_RECORDING",
            CoreError::InvalidRecording(_) => "INVALID_RECORDING",
            CoreError::Cancelled => "CANCELLED",
            CoreError::Timeout(_) => "TIMEOUT",
            CoreError::Serial(_) => "SERIAL_ERROR",
            CoreError::Io(_) => "IO_ERROR",
            CoreError::Config(_) => "CONFIG_ERROR",
            CoreError::Other(_) => "INTERNAL_ERROR",
        };
        Self {
            code: code.into(),
            message: error.to_string(),
        }
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
pub type CommandResult<T> = Result<T, AppError>;
