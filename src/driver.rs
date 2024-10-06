use std::{
    mem,
    path::{Path, PathBuf},
};

use oxc::{
    ast::{ast::Program, Trivias},
    codegen::{CodeGenerator, CodegenOptions, CodegenReturn, CommentOptions},
    diagnostics::OxcDiagnostic,
    mangler::{MangleOptions, Mangler},
    minifier::CompressOptions,
    parser::ParseOptions,
    span::SourceType,
    transformer::{EnvOptions, Targets, TransformOptions},
    CompilerInterface,
};

pub fn default_transformer_options() -> TransformOptions {
    let mut options = TransformOptions::from_preset_env(&EnvOptions {
        targets: Targets::from_query("chrome 51"),
        ..EnvOptions::default()
    })
    .unwrap();
    // `object_rest_spread` is not ready
    options.es2018.object_rest_spread = None;
    // Enables `only_remove_type_imports` avoiding removing all unused imports
    options.typescript.only_remove_type_imports = true;
    options
}

use crate::Diagnostic;

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct Driver {
    // options
    pub transform: Option<TransformOptions>,
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
        self.printed = ret.source_text;
    }

    fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            parse_regular_expression: true,
            allow_return_outside_function: true,
            ..ParseOptions::default()
        }
    }

    fn transform_options(&self) -> Option<TransformOptions> {
        self.transform.clone()
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

    fn codegen<'a>(
        &self,
        program: &Program<'a>,
        source_text: &'a str,
        _source_path: &Path,
        trivias: &Trivias,
        mangler: Option<Mangler>,
        options: CodegenOptions,
    ) -> CodegenReturn {
        let mut codegen = CodeGenerator::new().with_options(options).with_mangler(mangler);
        if self.compress_options().is_none() {
            let comment_options = CommentOptions { preserve_annotate_comments: true };
            codegen = codegen.enable_comment(source_text, trivias.clone(), comment_options);
        };
        codegen.build(program)
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
