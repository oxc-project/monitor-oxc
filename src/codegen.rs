use std::{fs, path::Path};

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
};

use crate::{Diagnostic, NodeModulesRunner, Source};

pub struct CodegenRunner;

impl CodegenRunner {
    pub fn run(self, runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Codegen");
        let diagnostics = runner
            .files
            .iter()
            .filter_map(|source| if let Err(d) = Self::test(source) { Some(d) } else { None })
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        NodeModulesRunner::run_runtime_test()
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let Source { path, source_type, source_text } = source;
        let source_text2 = Self::codegen(path, source_text, *source_type)?;
        // Idempotency test
        let source_text3 = Self::codegen(path, &source_text2, *source_type)?;

        if source_text2 != source_text3 {
            return Err(Diagnostic {
                case: "Codegen idempotency",
                path: path.clone(),
                message: NodeModulesRunner::print_diff(&source_text2, &source_text3),
            });
        }

        // Write js files for runtime test
        if source_type.is_javascript() {
            fs::write(path, source_text3).unwrap();
        }
        Ok(())
    }

    fn codegen(
        path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        let allocator = Allocator::default();
        let ParserReturn { program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type)
                .allow_return_outside_function(true)
                .parse();
        if !errors.is_empty() {
            let message = errors
                .into_iter()
                .map(|e| e.with_source_code(source_text.to_string()).to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(Diagnostic {
                case: "Codegen Parse Error",
                path: path.to_path_buf(),
                message,
            });
        }
        Ok(CodeGenerator::new()
            .enable_comment(
                source_text,
                trivias,
                CommentOptions { preserve_annotate_comments: true },
            )
            .build(&program)
            .source_text)
    }
}
