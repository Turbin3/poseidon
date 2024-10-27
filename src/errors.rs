use anchor_lang::error;
use thiserror::*;

#[derive(Debug, Error)]
pub enum PoseidonError {
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("Keyword type {0} is not supported")]
    KeyWordTypeNotSupported(String),
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
    #[error("expected a type reference")]
    TypeReferenceNotFound,
    #[error("expected a TS literal type")]
    TSLiteralTypeNotFound,
    #[error("expected a numeric literal for TS literal type")]
    NumericLiteralNotFound,
    #[error("expected a Atom type")]
    AtomNotFound,
}
