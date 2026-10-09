use oxc::ast::ast::{Comment, CommentKind};

use crate::{Case, Diagnostic, Driver, NodeModulesRunner, Source};

pub struct CodegenRunner;

impl Case for CodegenRunner {
    fn name(&self) -> &'static str {
        "Codegen"
    }

    fn driver(&self) -> Driver {
        Driver::default()
    }

    fn idempotency_test(
        &self,
        source: &Source,
    ) -> Result<(String, Option<String>), Vec<Diagnostic>> {
        let mut output = String::new();
        let mut diagnostics = Vec::new();
        for preserve_parens in [true, false] {
            match self.test_with_parens(source, preserve_parens) {
                // Keep the default mode's output for the runtime test.
                Ok(printed) if preserve_parens => output = printed,
                Ok(_) => {}
                Err(errors) => diagnostics.extend(errors.into_iter().map(|mut error| {
                    error.case = self.name();
                    error.message = format!("preserve_parens={preserve_parens}: {}", error.message);
                    error
                })),
            }
        }
        if diagnostics.is_empty() { Ok((output, None)) } else { Err(diagnostics) }
    }
}

impl CodegenRunner {
    fn driver_with_parens(&self, preserve_parens: bool) -> Driver {
        Driver {
            preserve_parens: Some(preserve_parens),
            comments: Some(Vec::new()),
            ..self.driver()
        }
    }

    fn test_with_parens(
        &self,
        source: &Source,
        preserve_parens: bool,
    ) -> Result<String, Vec<Diagnostic>> {
        let Source { path, source_type, source_text } = source;
        let mut driver = self.driver_with_parens(preserve_parens);
        let pass1 = driver.run(path, source_text, *source_type)?;
        let original_comments = normalized_comments(driver.comments.as_ref().unwrap(), source_text);
        let pass2 = driver.run(path, &pass1, *source_type)?;
        let printed_comments = normalized_comments(driver.comments.as_ref().unwrap(), &pass1);

        let mut diagnostics = Vec::new();
        if original_comments != printed_comments {
            diagnostics.push(Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!(
                    "Comments changed after codegen (original: {}, printed: {}).\n{}",
                    original_comments.len(),
                    printed_comments.len(),
                    NodeModulesRunner::get_diff(
                        &original_comments.join("\n"),
                        &printed_comments.join("\n"),
                        false,
                    ),
                ),
            });
        }
        if pass1 != pass2 {
            diagnostics.push(Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!(
                    "Codegen is not idempotent.\n{}",
                    NodeModulesRunner::get_diff(&pass1, &pass2, false),
                ),
            });
        }
        // Equal passes also guarantee that the second print preserved the comments.
        if diagnostics.is_empty() { Ok(pass1) } else { Err(diagnostics) }
    }
}

fn normalized_comments(comments: &[Comment], source_text: &str) -> Vec<String> {
    let mut texts = comments
        .iter()
        .map(|comment| {
            // HTML close comments can become ordinary line comments when moved
            // after code. Their content must still survive.
            let text = if matches!(comment.kind, CommentKind::HtmlOpen | CommentKind::HtmlClose) {
                format!("//{}", comment.content_span().source_text(source_text))
            } else {
                comment.span.source_text(source_text).to_string()
            };
            // Codegen reindents multiline comments and escapes script closing tags.
            let text = text.replace("<\\/", "</");
            if comment.is_multiline_block() {
                text.replace("\r\n", "\n")
                    .replace(['\r', '\u{2028}', '\u{2029}'], "\n")
                    .split('\n')
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                text
            }
        })
        .collect::<Vec<_>>();
    // Attachment can move comments. Compare a multiset so duplicates still count.
    texts.sort_unstable();
    texts
}

#[cfg(test)]
mod tests {
    use oxc::{
        CompilerInterface,
        allocator::Allocator,
        ast::ast::{Expression, Statement},
        parser::Parser,
        span::SourceType,
    };

    use super::{CodegenRunner, normalized_comments};
    use crate::{Case, Source};

    fn source(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            source_type: SourceType::from_path(path).unwrap(),
            source_text: text.into(),
        }
    }

    fn comments(text: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, text, SourceType::mjs()).parse();
        assert!(parsed.diagnostics.is_empty());
        normalized_comments(&parsed.program.comments, text)
    }

    #[test]
    fn prints_every_comment_with_parens_on_and_off() {
        let source = source(
            "fixture.js",
            r"// leading
/*! legal */
/** jsdoc */
export function value(input) {
    /* repeated */
    const values = [/* element */ (input), /* after element */]; // trailing
    consume(/* argument */ values);
    const empty = { /* dangling */ };
    const result = /* @__PURE__ */ factory();
    /* repeated */
    return { /* property */ value: input };
}
// eof
",
        );
        let expected = comments(&source.source_text);
        assert_eq!(expected.len(), 13);
        for preserve_parens in [true, false] {
            let output = CodegenRunner.test_with_parens(&source, preserve_parens).unwrap();
            assert_eq!(comments(&output), expected, "preserve_parens={preserve_parens}");
        }
    }

    #[test]
    fn idempotency_covers_javascript_typescript_and_jsx() {
        for (path, text) in [
            ("fixture.js", "export const value = ((1 + 2)) * (3 + 4);"),
            ("fixture.js", "export const value = (() => ({ value: (1) }))();"),
            ("fixture.js", "export const value = (object?.method)();"),
            ("fixture.js", "class C extends (/*#__PURE__*/ factory().annotations({})) {}"),
            ("fixture.js", "const value = new (/*#__PURE__*/ factory())();"),
            ("fixture.ts", "export type T = ((string | number))[];"),
            ("fixture.ts", "export const value = ((input as number)) + 1;"),
            ("fixture.jsx", "export const value = (<div>{(1 + 2)}</div>);"),
        ] {
            let source = source(path, text);
            for preserve_parens in [true, false] {
                CodegenRunner.test_with_parens(&source, preserve_parens).unwrap();
            }
            let (output, note) = CodegenRunner.idempotency_test(&source).unwrap();
            assert_eq!(output, CodegenRunner.test_with_parens(&source, true).unwrap());
            assert!(note.is_none());
        }
    }

    #[test]
    fn preserve_parens_configures_the_parser_ast() {
        let allocator = Allocator::default();
        for preserve_parens in [true, false] {
            let driver = CodegenRunner.driver_with_parens(preserve_parens);
            let parsed = driver.parse(&allocator, "const value = ((1));", SourceType::mjs());
            let Statement::VariableDeclaration(declaration) = &parsed.program.body[0] else {
                panic!("expected a variable declaration");
            };
            assert_eq!(
                matches!(
                    declaration.declarations[0].init.as_ref().unwrap(),
                    Expression::ParenthesizedExpression(_)
                ),
                preserve_parens,
            );
        }
    }

    #[test]
    fn reports_failures_for_both_paren_modes() {
        let source = source("fixture.js", "export const value = ;");
        let errors = CodegenRunner.idempotency_test(&source).unwrap_err();
        for preserve_parens in [true, false] {
            assert!(errors.iter().any(|error| {
                error.case == "Codegen"
                    && error.path == source.path
                    && error.message.starts_with(&format!("preserve_parens={preserve_parens}:"))
            }));
        }
    }

    #[test]
    fn comment_comparison_counts_duplicates_and_ignores_comment_like_strings() {
        let original = comments("/* repeated */ const value = '/* repeated */'; /* repeated */");
        assert_eq!(original.len(), 2);
        assert_ne!(original, comments("/* repeated */ const value = '/* repeated */';"));
        assert_ne!(original, comments("/* repeated */ /* repeated */ /* repeated */"));
        assert_ne!(original, comments("/* repeated */ /* changed */"));
        assert_eq!(comments("/* one */ // two\n"), comments("// two\n/* one */"));
    }

    #[test]
    fn comment_comparison_allows_reindentation_and_script_tag_escaping() {
        assert_eq!(
            comments("/* first\r\n    second\r\n    </ScRiPt> */"),
            comments("/* first\n\tsecond\n\t<\\/ScRiPt> */"),
        );
        let source =
            source("fixture.js", "/* first\n    second\n    </script> */\nexport const value = 1;");
        CodegenRunner.idempotency_test(&source).unwrap();
    }

    #[test]
    fn preserves_html_comment_contents() {
        for text in ["<!-- leading\nconsume();\n--> trailing\n", "--> #__PURE__\nfactory();"] {
            let source = source("fixture.cjs", text);
            CodegenRunner.idempotency_test(&source).unwrap();
        }
    }
}
