use thiserror::*;

#[derive(Debug, Error)]
pub enum PoseidonError {
    #[error("Invalid type: {0}")]
    InvalidType(String)
}