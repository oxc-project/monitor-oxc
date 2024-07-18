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
        let Source { path, source_type, source_text } = source;
        let source_text2 = Self::transform(path, source_text, *source_type)?;

        // Idempotency test
        let source_text3 = Self::transform(path, &source_text2, *source_type)?;

        if source_text2 != source_text3 {
            return Err(Diagnostic {
                case: "Transform idempotency",
                path: path.clone(),
                message: NodeModulesRunner::print_diff(&source_text2, &source_text3),
            });
        }

        // Write js files for runtime test
        let new_extension = path.extension().unwrap().to_string_lossy().replace('t', "j");
        let new_path = path.with_extension(new_extension);
        fs::write(new_path, source_text3).unwrap();
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
