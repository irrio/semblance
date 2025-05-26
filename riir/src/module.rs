mod bin;
mod builder;
mod repr;

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub use bin::{WasmDecodeError, WasmDecodeResult};
pub use repr::*;

#[derive(Debug)]
pub enum WasmReadError {
    Io(io::Error),
    Decode(WasmDecodeError),
}

impl repr::WasmModule {
    pub fn read(path: &Path) -> Result<Self, WasmReadError> {
        let mut f = File::open(path).map_err(WasmReadError::Io)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(WasmReadError::Io)?;
        let bytes = buf.into_boxed_slice();
        bin::decode(&bytes).map_err(WasmReadError::Decode)
    }

    pub fn decode(bytes: &[u8]) -> WasmDecodeResult<Self> {
        bin::decode(bytes)
    }
}
