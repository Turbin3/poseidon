use anyhow::{anyhow, Result};
use regex::Regex;

pub fn extract_accounts_structs(input: &str) -> Vec<String> {
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

pub fn reorder_struct(input: &str) -> Result<(String, String)> {
    let field_regex = Regex::new(
        r"(?ms)^(?P<attrs>(\s*#\[[^\]]*\](\s*|.*?))*?)\s*pub\s+(?P<name>\w+):\s+(?P<type>[^\n]+),",
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

pub fn replace_struct(code: &str, struct_header: &str, new_struct: &str) -> String {
    let struct_regex = Regex::new(&format!(
        r"(?ms)^{}.*?(}})",
        regex::escape(struct_header.trim())
    ))
    .unwrap();

    struct_regex.replace(code, new_struct).to_string()
}
