use std::{fs, path::Path};

use walkdir::WalkDir;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
};

pub struct CodegenRunner;

impl CodegenRunner {
    fn codegen(&self, path: &Path, source_text: &str, source_type: SourceType) -> String {
        let allocator = Allocator::default();
        let ParserReturn { program, errors, trivias, .. } =
            Parser::new(&allocator, source_text, source_type).parse();
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

    pub fn run(self) {
        for entry in WalkDir::new("node_modules") {
            let dir_entry = entry.unwrap();
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(source_type) = SourceType::from_path(path) else {
                continue;
            };
            let source_text = fs::read_to_string(path).unwrap();
            let source_text2 = self.codegen(path, &source_text, source_type);

            // Idempotency test
            let source_text3 = self.codegen(path, &source_text2, source_type);
            if source_text2 != source_text {
                println!("{}", source_text);
                panic!("Idempotency test failed: {path:?}");
            }

            // Write js files for runtime test
            if source_type.is_javascript() {
                fs::write(path, source_text3).unwrap();
            }
        }
    }
}
