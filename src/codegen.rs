use std::{fs, path::Path};

use console::Style;
use similar::{ChangeTag, TextDiff};
use walkdir::WalkDir;

use oxc::{
    allocator::Allocator,
    codegen::{CodeGenerator, CommentOptions},
    parser::{Parser, ParserReturn},
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

fn print_diff(origin_string: &str, expected_string: &str) {
    let diff = TextDiff::from_lines(expected_string, origin_string);
    for change in diff.iter_all_changes() {
        let (sign, style) = match change.tag() {
            ChangeTag::Delete => ("-", Style::new().red()),
            ChangeTag::Insert => ("+", Style::new().green()),
            ChangeTag::Equal => continue, // (" ", Style::new()),
        };
        println!("{}{}", style.apply_to(sign).bold(), style.apply_to(change))
    }
}

pub struct CodegenRunner;

impl CodegenRunner {
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

    pub fn run(self) {
        self.run_impl(Self::run_codegen);
        self.run_impl(Self::run_transform);
    }

    pub fn run_impl(&self, func: impl Fn(&CodegenRunner, &Path, SourceType)) {
        for entry in WalkDir::new("node_modules")
            .into_iter()
            .filter_entry(|e| !(e.path().is_dir() && e.path().ends_with(".pnpm")))
        {
            let dir_entry = entry.unwrap();
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(source_type) = SourceType::from_path(path) else {
                continue;
            };
            func(self, path, source_type)
        }
    }

    pub fn run_codegen(&self, path: &Path, source_type: SourceType) {
        let source_text = fs::read_to_string(path).unwrap();
        let source_text2 = self.codegen(path, &source_text, source_type);
        // Idempotency test
        let source_text3 = self.codegen(path, &source_text2, source_type);

        if source_text2 != source_text3 {
            print_diff(&source_text2, &source_text3);
            panic!("Codegen idempotency test failed: {path:?}");
        }

        // Write js files for runtime test
        if source_type.is_javascript() {
            fs::write(path, source_text3).unwrap();
        }
    }

    pub fn run_transform(&self, path: &Path, source_type: SourceType) {
        let source_text = fs::read_to_string(path).unwrap();
        let source_text2 = self.transform(path, &source_text, source_type);

        // Idempotency test
        let source_text3 = self.transform(path, &source_text2, source_type);

        if source_text2 != source_text3 {
            print_diff(&source_text2, &source_text3);
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
