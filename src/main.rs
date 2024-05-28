use std::path::Path;
mod rs_types;
mod ts_types;
// mod rs_type1;
mod errors;
mod transpiler;

use anyhow::Result;
use swc_common::{
    self,
    errors::{ColorConfig, Handler},
    sync::Lrc,
    SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax};
use transpiler::transpile;

fn main() -> Result<()> {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    // Real usage
    // let fm = cm
    //     .load_file(Path::new("counter.ts"))
    //     .expect("failed to load test.ts");

    // let fm = cm
    //     .load_file(Path::new("vault.ts"))
    //     .expect("failed to load test.ts");

    let fm = cm
        .load_file(Path::new("escrow.ts"))
        .expect("failed to load test.ts");

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

    transpile(&module)?;
    Ok(())
}
