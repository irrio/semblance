mod bin;
mod builder;
#[macro_use]
mod repr;
mod err;
mod valid;

use std::{fs::File, io::Read, path::Path};

pub use bin::{WasmDecodeError, WasmDecodeResult};
pub use err::{WasmFromBytesError, WasmReadError};
pub use repr::*;
pub use valid::{WasmValidationError, WasmValidationResult, validate};

impl repr::WasmModule {
    pub fn read(path: &Path) -> Result<Self, WasmReadError> {
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let bytes = buf.into_boxed_slice();
        let wmod = bin::decode(&bytes)?;
        let valid = validate(wmod)?;
        Ok(valid)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WasmFromBytesError> {
        let wmod = bin::decode(&bytes)?;
        let valid = validate(wmod)?;
        Ok(valid)
    }
}
