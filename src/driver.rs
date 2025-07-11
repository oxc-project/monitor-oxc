use std::{
    mem,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use oxc::{
    CompilerInterface,
    allocator::Allocator,
    codegen::{Codegen, CodegenOptions, CodegenReturn, CommentOptions},
    diagnostics::OxcDiagnostic,
    mangler::MangleOptions,
    minifier::{CompressOptions, Compressor},
    parser::{ParseOptions, Parser, ParserReturn},
    span::SourceType,
    transformer::TransformOptions,
};

use crate::Diagnostic;

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct Driver {
    // options
    pub transform: Option<TransformOptions>,
    pub compress: Option<CompressOptions>,
    pub dce: bool,
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
            })
            .filter(|d| d.message != "The keyword 'await' is reserved");
        self.errors.extend(errors);
    }

    fn after_parse(&mut self, parser_return: &mut ParserReturn) -> ControlFlow<()> {
        parser_return.errors = mem::take(&mut parser_return.errors).into_iter().filter(|e| {
            e.message != "`await` is only allowed within async functions and at the top levels of modules"
        }).collect::<Vec<_>>();
        ControlFlow::Continue(())
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

    fn transform_options(&self) -> Option<&TransformOptions> {
        self.transform.as_ref()
    }

    fn compress_options(&self) -> Option<CompressOptions> {
        self.compress.clone()
    }

    fn mangle_options(&self) -> Option<MangleOptions> {
        self.mangle.then(MangleOptions::default)
    }

    fn codegen_options(&self) -> Option<CodegenOptions> {
        Some(CodegenOptions {
            minify: self.remove_whitespace,
            comments: if self.compress.is_some() {
                CommentOptions::disabled()
            } else {
                CommentOptions::default()
            },
            source_map_path: self.compress.is_none().then(|| self.path.clone()),
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
        if self.dce {
            return Ok(Self::dce(source_text, source_type));
        }
        self.path = source_path.to_path_buf();
        let mut source_type = source_type;
        if source_path.extension().unwrap() == "js" {
            source_type = source_type.with_jsx(source_type.is_javascript()).with_unambiguous(true);
        }
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

    pub fn dce(source_text: &str, source_type: SourceType) -> String {
        let allocator = Allocator::default();
        let mut ret = Parser::new(&allocator, source_text, source_type).parse();
        let program = &mut ret.program;
        Compressor::new(&allocator, CompressOptions::default()).dead_code_elimination(program);
        Codegen::new().build(program).code
    }
}
