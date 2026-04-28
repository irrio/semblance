use std::io;

use crate::module::{WasmDecodeError, WasmValidationError};

#[derive(Debug)]
pub enum WasmReadError {
    Io(io::Error),
    Decode(WasmDecodeError),
    Validation(WasmValidationError),
}

impl From<io::Error> for WasmReadError {
    fn from(value: io::Error) -> Self {
        WasmReadError::Io(value)
    }
}

impl From<WasmDecodeError> for WasmReadError {
    fn from(value: WasmDecodeError) -> Self {
        WasmReadError::Decode(value)
    }
}

impl From<WasmValidationError> for WasmReadError {
    fn from(value: WasmValidationError) -> Self {
        WasmReadError::Validation(value)
    }
}

#[derive(Debug)]
pub enum WasmFromBytesError {
    Decode(WasmDecodeError),
    Validation(WasmValidationError),
}

impl From<WasmDecodeError> for WasmFromBytesError {
    fn from(value: WasmDecodeError) -> Self {
        WasmFromBytesError::Decode(value)
    }
}

impl From<WasmValidationError> for WasmFromBytesError {
    fn from(value: WasmValidationError) -> Self {
        WasmFromBytesError::Validation(value)
    }
}
