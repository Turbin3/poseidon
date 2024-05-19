use anchor_lang::error;
use thiserror::*;

#[derive(Debug, Error)]
pub enum PoseidonError {
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("expected a expr in {0} call")]
    NoExprInCall(String),
    #[error("expected a member in the expr of a {0} call")]
    NoMemInExprOfCall(String)
}