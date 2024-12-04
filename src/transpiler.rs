use core::panic;
use rust_format::{Formatter, PrettyPlease};
use std::{collections::HashMap, fs};

use crate::rs_types::{ProgramAccount, ProgramModule};
use anyhow::{anyhow, Error, Result};
use regex::Regex;
use swc_ecma_ast::*;

pub fn transpile(module: &Module, output_file_name: &String) -> Result<()> {
    let mut imports = vec![];
    let mut accounts: HashMap<String, ProgramAccount> = HashMap::new();
    let mut program_class: Option<ClassExpr> = None;
    let mut custom_types: HashMap<String, ProgramAccount> = HashMap::new();
    let mut program = ProgramModule::new();
    let mut stack: Vec<&ModuleItem> = module.body.iter().collect();

    while let Some(item) = stack.pop() {
        match item {
            // Extract imports
            ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) => {
                let src = import_decl.src.value.to_string();
                let mut names = Vec::new();
                for specifier in &import_decl.specifiers {
                    if let ImportSpecifier::Named(named_specifier) = specifier {
                        names.push(named_specifier.local.sym.to_string());
                    }
                }
                imports.push((src, names));
            }
            // Extract program class
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(default_export_decl)) => {
                program_class = match default_export_decl.clone().decl.class() {
                    Some(p) => Some(p),
                    None => panic!("Default export must be a Class"),
                };
            }
            // Extract custom accounts
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(class_decl)) => {
                match class_decl.clone().decl {
                    Decl::TsInterface(interface) => {
                        let custom_account = ProgramAccount::from_ts_expr(*interface);
                        custom_types.insert(custom_account.name.clone(), custom_account.clone());
                        accounts.insert(custom_account.name.clone(), custom_account.clone());
                    }
                    _ => panic!("Invalid export statement"),
                }
            }
            _ => panic!("Invalid syntax, cannot match: {:?}", item),
        }
    }

    program.accounts = accounts.into_values().collect();
    program.custom_types.clone_from(&custom_types);

    match program_class {
        Some(c) => {
            program.populate_from_class_expr(&c, &custom_types)?;
        }
        None => panic!("Program class undefined"),
    }
    let serialized_program = program.to_tokens()?.to_string();

    let mut formatted_program = PrettyPlease::default().format_str(&serialized_program)?;

    let extracted_account_struct = extract_accounts_structs(&formatted_program);

    for account_struct in extracted_account_struct {
        let (header, reordered_account_struct) = reorder_struct(&account_struct)?;

        formatted_program = replace_struct(&formatted_program, &header, &reordered_account_struct);
    }

    fs::write(
        &output_file_name,
        PrettyPlease::default().format_str(formatted_program)?,
    )?;
    Ok(())
}

fn extract_accounts_structs(input: &str) -> Vec<String> {
    // Regex to capture structs with #[derive(Accounts)]
    let regex = Regex::new(
        r"(?s)#\[derive\(Accounts\)\](?:\s*#\[[^\]]*\])?\s*pub struct (\w+<'?\w*>) \{(.*?)\}",
    )
    .unwrap();

    regex
        .captures_iter(input)
        .map(|cap| format!("pub struct {} {{\n{}\n}}", &cap[1], &cap[2]))
        .collect()
}

fn reorder_struct(input: &str) -> Result<(String, String)> {
    let field_regex = Regex::new(
        r"(?ms)^(?P<attrs>(\s*#\[[^\]]+\]\s*)*)\s*pub\s+(?P<name>\w+):\s+(?P<type>[^\n]+),",
    )
    .unwrap();

    let mut init_fields = Vec::new();
    let mut init_if_needed_fields = Vec::new();
    let mut other_fields = Vec::new();

    for cap in field_regex.captures_iter(input) {
        let attrs = cap.name("attrs").unwrap().as_str().trim();
        let field = format!(
            "{}\n    pub {}: {},",
            attrs,
            cap.name("name").unwrap().as_str(),
            cap.name("type").unwrap().as_str()
        );

        if attrs.contains("init") && !attrs.contains("init_if_needed") {
            init_fields.push(field);
        } else if attrs.contains("init_if_needed") {
            init_if_needed_fields.push(field);
        } else {
            other_fields.push(field);
        }
    }

    let mut reordered_fields = String::new();
    for field in init_fields {
        reordered_fields.push_str(&field);
        reordered_fields.push('\n');
    }
    for field in init_if_needed_fields {
        reordered_fields.push_str(&field);
        reordered_fields.push('\n');
    }
    for field in other_fields {
        reordered_fields.push_str(&field);
        reordered_fields.push('\n');
    }

    let struct_regex = Regex::new(r"(?ms)^pub\s+struct\s+\w+<'\w+>\s*\{").unwrap();
    if let Some(header) = struct_regex.find(input) {
        Ok((
            header.as_str().to_string(),
            format!("{}\n{}\n}}", header.as_str(), reordered_fields),
        ))
    } else {
        Err(anyhow!("Invalid struct input"))
    }
}

fn replace_struct(code: &str, struct_header: &str, new_struct: &str) -> String {
    let struct_regex = Regex::new(&format!(
        r"(?ms)^{}.*?\{{.*?^\}}",
        regex::escape(struct_header.trim())
    ))
    .unwrap();

    struct_regex.replace(code, new_struct).to_string()
}
