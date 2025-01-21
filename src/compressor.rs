use oxc::minifier::CompressOptions;

use crate::{Case, Driver, Source};

pub struct CompressorRunner;

impl Case for CompressorRunner {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    /// The compressor changes cjs module syntaxes,
    /// which breaks `cjs-module-lexer`.
    /// e.g. `cjs-module-lexer` cannot detect `enumerable: !0`.
    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver { compress: Some(CompressOptions::default()), ..Driver::default() }
    }
}
