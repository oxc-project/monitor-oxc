use std::{
    mem,
    path::{Path, PathBuf},
};

use oxc::{
    codegen::{CodegenOptions, CodegenReturn},
    diagnostics::OxcDiagnostic,
    mangler::MangleOptions,
    minifier::CompressOptions,
    parser::ParseOptions,
    span::SourceType,
    transformer::TransformOptions,
    CompilerInterface,
};

use crate::Diagnostic;

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct Driver {
    // options
    pub transform: bool,
    pub compress: bool,
    pub mangle: bool,
    pub remove_whitespace: bool,
    // states
    pub printed: String,
    pub path: PathBuf,
    pub errors: Vec<OxcDiagnostic>,
}

impl CompilerInterface for Driver {
    fn handle_errors(&mut self, errors: Vec<OxcDiagnostic>) {
        let errors = errors
            .into_iter()
            .filter(|d| !d.message.starts_with("Flow is not supported"))
            // ignore `import lib = require(...);` syntax errors for transforms
            .filter(|d| {
                !d.message
                    .contains("add @babel/plugin-transform-modules-commonjs to your Babel config")
            });
        self.errors.extend(errors);
    }

    fn after_codegen(&mut self, ret: CodegenReturn) {
        self.printed = ret.code;
    }

    fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            parse_regular_expression: true,
            allow_return_outside_function: true,
            ..ParseOptions::default()
        }
    }

    fn transform_options(&self) -> Option<TransformOptions> {
        self.transform.then(|| {
            let mut options = TransformOptions::enable_all();
            // Turns off the refresh plugin because it is never idempotent
            options.react.refresh = None;
            // Enables `only_remove_type_imports` avoiding removing all unused imports
            options.typescript.only_remove_type_imports = true;
            // Unfinished plugins
            options.es2018.object_rest_spread = None;

            options
        })
    }

    fn compress_options(&self) -> Option<CompressOptions> {
        self.compress.then(CompressOptions::default)
    }

    fn mangle_options(&self) -> Option<MangleOptions> {
        self.mangle.then(MangleOptions::default)
    }

    fn codegen_options(&self) -> Option<CodegenOptions> {
        Some(CodegenOptions {
            minify: self.remove_whitespace,
            comments: !self.compress,
            source_map_path: (!self.compress).then(|| self.path.clone()),
            ..CodegenOptions::default()
        })
    }
}

impl Driver {
    pub fn run(
        &mut self,
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Vec<Diagnostic>> {
        self.path = source_path.to_path_buf();
        self.compile(source_text, source_type, source_path);
        if self.errors.is_empty() {
            Ok(mem::take(&mut self.printed))
        } else {
            let errors = mem::take(&mut self.errors)
                .into_iter()
                .map(|error| error.with_source_code(source_text.to_string()))
                .map(|error| Diagnostic {
                    case: "Error",
                    path: source_path.to_path_buf(),
                    message: format!("{error:?}"),
                })
                .collect();
            Err(errors)
        }
    }
}
