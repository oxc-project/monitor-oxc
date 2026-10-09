use std::{
    mem,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use oxc::{
    CompilerInterface,
    allocator::Allocator,
    ast::ast::Comment,
    codegen::{Codegen, CodegenOptions, CodegenReturn, CommentOptions},
    diagnostics::{Diagnostics, OxcDiagnostic},
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
    pub preserve_parens: Option<bool>,
    // states
    pub printed: String,
    pub path: PathBuf,
    pub errors: Vec<OxcDiagnostic>,
    // Only collected by the codegen case, which checks comment preservation.
    pub comments: Option<Vec<Comment>>,
}

impl CompilerInterface for Driver {
    fn handle_errors(&mut self, errors: Diagnostics) {
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
        if let Some(comments) = &mut self.comments {
            comments.clear();
            comments.extend_from_slice(&parser_return.program.comments);
        }
        parser_return.diagnostics = mem::take(&mut parser_return.diagnostics)
            .into_iter()
            .filter(|e| {
                e.message
                    != "`await` is only allowed within async functions and at the top levels of modules"
            })
            .collect();
        ControlFlow::Continue(())
    }

    fn after_codegen(&mut self, ret: CodegenReturn<'_>) {
        self.printed = ret.code;
    }

    fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            parse_regular_expression: true,
            allow_return_outside_function: true,
            preserve_parens: self
                .preserve_parens
                .unwrap_or(ParseOptions::default().preserve_parens),
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
        self.mangle.then(|| MangleOptions {
            // Keep `exports` / `module` wrapper bindings so Node's cjs-module-lexer
            // still detects named exports of mangled CommonJS / UMD packages
            // (`import { queue } from "async"`). See oxc-project/oxc#24041.
            reserved: ["exports", "module"].into_iter().map(Into::into).collect(),
            ..MangleOptions::default()
        })
    }

    fn codegen_options(&self) -> Option<CodegenOptions> {
        Some(CodegenOptions {
            minify: self.remove_whitespace,
            comments: if self.compress.is_some() {
                CommentOptions { annotation: true, ..CommentOptions::disabled() }
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
        Compressor::new(&allocator).dead_code_elimination(program, CompressOptions::dce());
        Codegen::new().build(program).code
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use oxc::{
        allocator::Allocator, minifier::CompressOptions, parser::Parser, span::SourceType,
        transformer::TransformOptions,
    };

    use super::Driver;

    #[test]
    fn parser_assigns_comment_owners() {
        let allocator = Allocator::default();
        let parsed = Parser::new(
            &allocator,
            "/* leading */ consume(/* argument */ 1); // trailing",
            SourceType::mjs(),
        )
        .parse();
        assert_eq!(parsed.program.comments.len(), 3);
        assert!(parsed.program.comments.iter().all(|comment| comment.attachment.is_some()));
    }

    #[test]
    fn comment_printing_is_idempotent_through_transform_and_mangle() {
        let source = "/* leading */ export function value(input) {\n\
            const values = [/* element */ input];\n\
            consume(/* argument */ values); // trailing\n\
            return { /* property */ value: input };\n\
            }";
        for transform in [false, true] {
            for mangle in [false, true] {
                let mut driver = Driver {
                    transform: transform.then(TransformOptions::default),
                    mangle,
                    ..Driver::default()
                };
                let output =
                    driver.run(Path::new("fixture.js"), source, SourceType::mjs()).unwrap();
                for comment in ["leading", "element", "argument", "trailing", "property"] {
                    assert_eq!(output.matches(comment).count(), 1, "{output}");
                }
                let second =
                    driver.run(Path::new("fixture.js"), &output, SourceType::mjs()).unwrap();
                assert_eq!(output, second);
            }
        }
    }

    #[test]
    fn dce_preserves_live_and_orphaned_comments() {
        let output = Driver::dce(
            "/* removed */ if (false) gone();\n/* live */ consume(/* argument */ 1);",
            SourceType::mjs(),
        );
        assert!(!output.contains("gone()"), "{output}");
        for comment in ["removed", "live", "argument"] {
            assert_eq!(output.matches(comment).count(), 1, "{output}");
        }
        assert!(output.find("/* live */").unwrap() < output.find("consume(").unwrap());
        assert!(output.find("consume(").unwrap() < output.find("/* argument */").unwrap());
    }

    #[test]
    fn pure_annotations_inside_generated_parentheses_are_idempotent() {
        for source in [
            "class C extends /*#__PURE__*/ factory().annotations({}) {}",
            "class C extends (/*#__PURE__*/ factory().annotations({})) {}",
            "class C extends // #__PURE__\nfactory() {}",
            "const x = /*#__PURE__*/ factory().value;",
            "const x = /*#__PURE__*/ factory()();",
            "const x = /*#__PURE__*/ new Factory().value;",
            "const x = /*#__PURE__*/ new Factory(arg).value;",
            "const x = new (/*#__PURE__*/ factory())();",
            "function f() { return /*#__PURE__*/ factory().value; }",
        ] {
            for remove_whitespace in [false, true] {
                let mut driver = Driver { remove_whitespace, ..Driver::default() };
                let output =
                    driver.run(Path::new("fixture.js"), source, SourceType::mjs()).unwrap();
                assert_eq!(output.matches("#__PURE__").count(), 1, "{output}");
                let second =
                    driver.run(Path::new("fixture.js"), &output, SourceType::mjs()).unwrap();
                assert_eq!(output, second, "{source}");
            }
        }
    }

    #[test]
    fn compression_preserves_annotation_comments_only() {
        let mut driver = Driver {
            compress: Some(CompressOptions::default()),
            remove_whitespace: true,
            ..Driver::default()
        };
        let output = driver
            .run(
                Path::new("fixture.js"),
                "export const value = /* @__PURE__ */ Symbol('value'); /* ordinary */",
                SourceType::default(),
            )
            .unwrap();

        assert!(output.contains("@__PURE__"));
        assert!(!output.contains("ordinary"));
    }
}
