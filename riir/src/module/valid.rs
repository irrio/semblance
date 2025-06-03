use super::WasmModule;

#[derive(Debug)]
pub enum WasmValidationError {}

pub type WasmValidationResult<T> = Result<T, WasmValidationError>;

pub fn validate(_wmod: &WasmModule) -> WasmValidationResult<()> {
    Ok(())
}
