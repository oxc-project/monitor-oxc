use std::{fs, path::Path};

use anyhow::Result;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

use crate::{NodeModulesRunner, Source};

pub struct TransformRunner;

impl TransformRunner {
    pub fn run(self, runner: &NodeModulesRunner) -> Result<()> {
        println!("Running Transformer");
        for source in &runner.files {
            self.test(source)?;
        }
        NodeModulesRunner::run_runtime_test()
    }

    fn test(&self, source: &Source) -> Result<()> {
        let Source { path, source_type, source_text } = source;
        let source_text2 = self.transform(path, source_text, *source_type)?;

        // Idempotency test
        let source_text3 = self.transform(path, &source_text2, *source_type)?;

        if source_text2 != source_text3 {
            NodeModulesRunner::print_diff(&source_text2, &source_text3);
            anyhow::bail!("Transform idempotency test failed: {path:?}");
        }

        // Write js files for runtime test
        if source_type.is_javascript() {
            let path = ["js", "mjs", "cjs"].iter().find_map(|ext| {
                let new_path = path.with_extension(ext);
                if new_path.is_file() {
                    Some(new_path)
                } else {
                    None
                }
            });
            if let Some(path) = path {
                fs::write(path, source_text3).unwrap();
            } else {
                // Maybe .d.ts file
            }
        }
        Ok(())
    }

    pub fn transform(
        &self,
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String> {
        let allocator = Allocator::default();

        let ParserReturn { mut program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type)
                .allow_return_outside_function(true)
                .parse();
        if !errors.is_empty() {
            for error in errors {
                println!("{:?}", error.with_source_code(source_text.to_string()));
            }
            anyhow::bail!("Expect no parse errors: {source_path:?}");
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
