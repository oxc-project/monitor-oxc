use std::{fs, path::Path};

use crate::{NodeModulesRunner, Source};

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
};

pub struct CodegenRunner {
    runner: NodeModulesRunner,
}

impl CodegenRunner {
    pub fn new() -> Self {
        Self { runner: NodeModulesRunner::new() }
    }

    pub fn run(self) {
        for source in &self.runner.files {
            self.test(source);
        }
        NodeModulesRunner::run_runtime_test();
    }

    fn test(&self, source: &Source) {
        let Source { path, source_type, source_text } = source;
        if source_text.starts_with("// @flow") {
            return;
        }
        let source_text2 = self.codegen(path, source_text, *source_type);
        // Idempotency test
        let source_text3 = self.codegen(path, &source_text2, *source_type);

        if source_text2 != source_text3 {
            NodeModulesRunner::print_diff(&source_text2, &source_text3);
            panic!("Codegen idempotency test failed: {path:?}");
        }

        // Write js files for runtime test
        if source_type.is_javascript() {
            fs::write(path, source_text3).unwrap();
        }
    }

    fn codegen(&self, path: &Path, source_text: &str, source_type: SourceType) -> String {
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
