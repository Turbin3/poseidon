use swc_ecma_ast::{TsType, TsTypeParamInstantiation};

use crate::errors::PoseidonError;
use anyhow::{Error, Ok, Result};
pub fn extract_ts_type(
    binding: Box<swc_ecma_ast::TsTypeAnn>,
) -> Result<(String, u32, bool), Error> {
    let ts_type: String;
    let length: u32;
    match binding.type_ann.as_ref() {
        TsType::TsTypeRef(_) => {
            let ident = binding
                .type_ann
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_name
                .as_ident()
                .ok_or(PoseidonError::IdentNotFound)?;

            if let Some(type_params) = &binding
                .type_ann
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_params
            {
                (ts_type, length) =
                    extract_name_and_len_with_type_params(ident.sym.as_ref(), type_params)?;
            } else {
                ts_type = String::from(ident.sym.to_string());
                length = 1;
            }

            Ok((ts_type, length, ident.optional))
        }
        _ => Err(PoseidonError::KeyWordTypeNotSupported(format!(
            "{:?}",
            binding.type_ann.as_ref()
        ))
        .into()),
    }
}

fn extract_name_and_len_with_type_params(
    primary_type_ident: &str,
    type_params: &Box<TsTypeParamInstantiation>,
) -> Result<(String, u32), Error> {
    let ts_type: String;
    let mut length: u32 = 0;
    match primary_type_ident {
        "Str" => {
            length += type_params.params[0]
                .as_ts_lit_type()
                .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                .lit
                .as_number()
                .ok_or(PoseidonError::NumericLiteralNotFound)?
                .value as u32;
            ts_type = String::from("Str");
        }
        "Vec" => {
            let vec_type_name = type_params.params[0]
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_name
                .as_ident()
                .ok_or(PoseidonError::IdentNotFound)?
                .sym
                .to_string();

            let vec_len = type_params.params[1]
                .as_ts_lit_type()
                .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                .lit
                .as_number()
                .ok_or(PoseidonError::NumericLiteralNotFound)?
                .value as u32;

            if let Some(type_params_layer) = &type_params.params[0]
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_params
            {
                let type_ident_layer = type_params.params[0]
                    .as_ts_type_ref()
                    .ok_or(PoseidonError::TypeReferenceNotFound)?
                    .type_name
                    .as_ident()
                    .ok_or(PoseidonError::IdentNotFound)?
                    .sym
                    .as_ref();

                // for multiple nesting support recursion can be used
                // (type_name_layer, length_layer) = extract_name_and_len_with_type_params(type_ident_layer, type_params_layer)?;

                if type_ident_layer == "Str" {
                    let string_length = type_params_layer.params[0]
                        .as_ts_lit_type()
                        .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                        .lit
                        .as_number()
                        .ok_or(PoseidonError::NumericLiteralNotFound)?
                        .value as u32;

                    length += vec_len * (4 + string_length);
                    ts_type = format!("Vec<Str>");
                } else {
                    return Err(PoseidonError::KeyWordTypeNotSupported(format!(
                        "{:?}",
                        primary_type_ident
                    ))
                    .into());
                }
            } else {
                length += vec_len;
                ts_type = format!("Vec<{}>", vec_type_name);
            }
        }
        _ => {
            return Err(
                PoseidonError::KeyWordTypeNotSupported(format!("{:?}", primary_type_ident)).into(),
            )
        }
    }

    Ok((ts_type, length))
}
