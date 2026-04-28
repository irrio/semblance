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
        let bytes = {
            let mut f = File::open(path)?;
            let meta = f.metadata()?;
            let mut buf = Vec::with_capacity(meta.len() as usize);
            f.read_to_end(&mut buf)?;
            buf.into_boxed_slice()
        };
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
