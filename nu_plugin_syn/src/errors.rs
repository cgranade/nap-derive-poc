use thiserror::Error;
use surf;

pub type Result<T> = core::result::Result<T, SynError>;

#[derive(Error, Debug)]
pub enum SynError {
    #[error("HTTP error {}", .0.status())]
    HttpError(surf::Error),

    #[error("Unknown Synology error")]
    Unknown,

    #[error("Unknown Synology error {code}")]
    MiscApiError {
        code: u32
    }
}

impl SynError {
    pub fn from_api_error_code(code: u32) -> Self {
        // TODO: 4xx error codes have different interpretations depending
        //       on what group of API endpoints led to the error.
        match code {
            100 => Self::Unknown,
            _ => SynError::MiscApiError { code }
        }
    }
}
