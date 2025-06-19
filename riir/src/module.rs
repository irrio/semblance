mod bin;
mod builder;
#[macro_use]
mod repr;
mod valid;

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub use bin::{WasmDecodeError, WasmDecodeResult};
pub use repr::*;
pub use valid::{WasmValidationError, WasmValidationResult};

#[derive(Debug)]
pub enum WasmReadError {
    Io(io::Error),
    Decode(WasmDecodeError),
    Validation(WasmValidationError),
}

impl repr::WasmModule {
    pub fn read(path: &Path) -> Result<Self, WasmReadError> {
        let mut f = File::open(path).map_err(WasmReadError::Io)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(WasmReadError::Io)?;
        let bytes = buf.into_boxed_slice();
        let wmod = bin::decode(&bytes).map_err(WasmReadError::Decode)?;
        wmod.validate().map_err(WasmReadError::Validation)
    }
}
