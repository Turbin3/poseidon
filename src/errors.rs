use anchor_lang::error;
use thiserror::*;

#[derive(Debug, Error)]
pub enum PoseidonError {
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("expected a Member type")]
    MemberNotFound,
    #[error("expected a Expr type")]
    ExprNotFound,
    #[error("expected a Ident type")]
    IdentNotFound,
    #[error("expected a Array type")]
    ArrayNotFound,
    #[error("expected a Call type")]
    CallNotFound,
}
