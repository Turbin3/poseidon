use core::panic;
use std::{fs, collections::HashMap};
use rust_format::{Formatter, PrettyPlease, RustFmt};

use swc_ecma_ast::*;
use crate::rs_types::{
    ProgramModule, 
    ProgramAccount
};

pub fn transpile(module: &Module) {
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
                    None => panic!("Default export must be a Class")
                };
            },
            // Extract custom accounts
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(class_decl)) => {
                match class_decl.clone().decl {
                    Decl::TsInterface(interface) => {
                        let custom_account = ProgramAccount::from_ts_expr(interface);
                        custom_types.insert(custom_account.name.clone(), custom_account.clone());
                        accounts.insert(custom_account.name.clone(), custom_account.clone());
                    },
                    _ => panic!("Invalid export statement")
                }
            }
            _ => panic!("Invalid syntax, cannot match: {:?}", item)
        }
    }
    
    program.accounts = accounts.into_values().collect();
    program.custom_types = custom_types.clone();
    // print!("{:#?}", program_class);
    match program_class {
        Some(c) => program.populate_from_class_expr(&c, &custom_types),
        None => panic!("Program class undefined")
    }
    let serialized_program = program.to_tokens();
    fs::write("escrow.rs", RustFmt::default().format_str(&serialized_program.to_string()).unwrap()).unwrap()
}

