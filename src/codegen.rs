use std::{fs, path::Path};

use anyhow::Result;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
};

use crate::{NodeModulesRunner, Source};

pub struct CodegenRunner;

impl CodegenRunner {
    pub fn run(self, runner: &NodeModulesRunner) -> Result<()> {
        println!("Running Codegen");
        for source in &runner.files {
            Self::test(source)?;
        }
        NodeModulesRunner::run_runtime_test()
    }

    fn test(source: &Source) -> Result<()> {
        let Source { path, source_type, source_text } = source;
        let source_text2 = Self::codegen(path, source_text, *source_type);
        // Idempotency test
        let source_text3 = Self::codegen(path, &source_text2, *source_type);

        if source_text2 != source_text3 {
            NodeModulesRunner::print_diff(&source_text2, &source_text3);
            anyhow::bail!("Codegen idempotency test failed: {path:?}");
        }

        // Write js files for runtime test
        if source_type.is_javascript() {
            fs::write(path, source_text3).unwrap();
        }
        Ok(())
    }

    fn codegen(path: &Path, source_text: &str, source_type: SourceType) -> String {
        let allocator = Allocator::default();
        let ParserReturn { program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type)
                .allow_return_outside_function(true)
                .parse();
        if !errors.is_empty() {
            for error in errors {
                println!("{:?}", error.with_source_code(source_text.to_string()));
            }
            panic!("Expect no parse errors: {path:?}");
        }
        CodeGenerator::new()
            .enable_comment(
                source_text,
                trivias,
                CommentOptions { preserve_annotate_comments: true },
            )
            .build(&program)
            .source_text
    }
}
