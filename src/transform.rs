use std::{fs, path::Path};

use crate::{NodeModulesRunner, Source};

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

pub struct TransformRunner {
    runner: NodeModulesRunner,
}

impl TransformRunner {
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
        let source_text2 = self.transform(path, &source_text, *source_type);

        // Idempotency test
        let source_text3 = self.transform(path, &source_text2, *source_type);

        if source_text2 != source_text3 {
            NodeModulesRunner::print_diff(&source_text2, &source_text3);
            panic!("Transform idempotency test failed: {path:?}");
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
    }

    pub fn transform(
        &self,
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> String {
        let allocator = Allocator::default();

        let ParserReturn { mut program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type)
                .allow_return_outside_function(true)
                .parse();
        if !errors.is_empty() {
            for error in errors {
                println!("{:?}", error.with_source_code(source_text.to_string()));
            }
            panic!("Expect no parse errors: {source_path:?}");
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

        source.replace(".mts", "").replace(".cts", "").replace(".ts", "")
    }
}
