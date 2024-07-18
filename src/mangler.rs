use std::{fs, path::Path};

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    mangler::ManglerBuilder,
    parser::{Parser, ParserReturn},
    span::SourceType,
};

use crate::{Diagnostic, NodeModulesRunner, Source};

pub struct ManglerRunner;

impl ManglerRunner {
    pub fn run(runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Mangler");
        runner.run(Self::test)
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let source_text = NodeModulesRunner::idempotency_test("mangler", source, Self::mangle)?;
        // Write js files for runtime test
        if source.source_type.is_javascript() {
            fs::write(&source.path, source_text).unwrap();
        }
        Ok(())
    }

    fn mangle(
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
                case: "Mangler Parse Error",
                path: path.to_path_buf(),
                message,
            });
        }
        let mangler = ManglerBuilder::default().debug(true).build(&program);
        Ok(CodeGenerator::new()
            .enable_comment(
                source_text,
                trivias,
                CommentOptions { preserve_annotate_comments: true },
            )
            .with_mangler(Some(mangler))
            .build(&program)
            .source_text)
    }
}
