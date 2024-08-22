use std::{
    mem,
    path::{Path, PathBuf},
};

use oxc::{
    codegen::CodegenOptions, diagnostics::OxcDiagnostic, mangler::MangleOptions,
    minifier::CompressOptions, parser::ParseOptions, span::SourceType,
    transformer::TransformOptions, CompilerInterface,
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
        self.errors
            .extend(errors.into_iter().filter(|d| !d.message.starts_with("Flow is not supported")));
    }

    fn after_codegen(&mut self, printed: String) {
        self.printed = printed;
    }

    fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            parse_regular_expression: true,
            allow_return_outside_function: true,
            ..ParseOptions::default()
        }
    }

    fn transform_options(&self) -> Option<TransformOptions> {
        self.transform.then(TransformOptions::default)
    }

    fn compress_options(&self) -> Option<CompressOptions> {
        self.compress.then(CompressOptions::default)
    }

    fn mangle_options(&self) -> Option<MangleOptions> {
        self.mangle.then(MangleOptions::default)
    }

    fn codegen_options(&self) -> Option<CodegenOptions> {
        Some(CodegenOptions { minify: self.remove_whitespace, ..CodegenOptions::default() })
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
