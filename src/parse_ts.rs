use std::path::Path;
use swc_common::{
    self,
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax};

pub fn parse_ts(input_file_name: &String) -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    // Real usage
    // let fm = cm
    //     .load_file(Path::new("counter.ts"))
    //     .expect("failed to load counter.ts");

    // let fm = cm
    //     .load_file(Path::new("vault.ts"))
    //     .expect("failed to load vault.ts");

    let fm = cm
        .load_file(Path::new(&input_file_name))
        .expect(&format!("failed to load {}", input_file_name));

    let lexer = Lexer::new(
        Syntax::Typescript(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let capturing = Capturing::new(lexer);

    let mut parser = Parser::new_from(capturing);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_typescript_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("Failed to parse module.");

    return module;
}
