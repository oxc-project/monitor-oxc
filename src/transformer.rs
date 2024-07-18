use std::{fs, path::Path};

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

use crate::{Diagnostic, NodeModulesRunner, Source};

pub struct TransformRunner;

impl TransformRunner {
    pub fn run(runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Transformer");
        runner.run(Self::test)
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let source_text =
            NodeModulesRunner::idempotency_test("transform", source, Self::transform)?;
        // Write files for runtime test
        let new_extension = source.path.extension().unwrap().to_string_lossy().replace('t', "j");
        let new_path = source.path.with_extension(new_extension);
        fs::write(new_path, source_text).unwrap();
        Ok(())
    }

    pub fn transform(
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        let allocator = Allocator::default();

        let ParserReturn { mut program, errors, trivias, .. } =
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
                case: "Transformer Parse Error",
                path: source_path.to_path_buf(),
                message,
            });
        }

        Transformer::new(
            &allocator,
            source_path,
            source_type,
            source_text,
            trivias.clone(),
            TransformOptions::default(),
        )
        .build(&mut program);

        let source = CodeGenerator::new()
            .enable_comment(
                source_text,
                trivias,
                CommentOptions { preserve_annotate_comments: true },
            )
            .build(&program)
            .source_text;

        Ok(source)
    }
}
