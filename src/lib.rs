use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use ignore::Walk;

use oxc::{
    allocator::Allocator,
    codegen::{Codegen, CodegenOptions},
    parser::Parser,
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

pub struct Runner {
    dirs: Vec<PathBuf>,
    count: usize,
}

impl Runner {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self { dirs, count: 0 }
    }

    pub fn run(mut self) -> ExitCode {
        for dir in self.dirs.clone() {
            println!("Processing {:?}", dir);
            self.walk(&dir);
        }

        if self.count > 0 {
            println!("Transformed {:?} files", self.count);
            ExitCode::SUCCESS
        } else {
            eprintln!("No files were transformed");
            ExitCode::FAILURE
        }
    }

    fn walk(&mut self, dir: &Path) {
        for entry in Walk::new(dir) {
            let dir_entry = entry.unwrap();
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(source_type) = SourceType::from_path(path) else {
                continue;
            };
            if source_type.is_typescript_definition() {
                continue;
            }
            let source_text = fs::read_to_string(path).unwrap();
            let source_text2 = self.transform(path, &source_text, source_type);

            let new_extension = path.extension().unwrap().to_str().unwrap().replace('t', "j");
            let new_path = path.with_extension(new_extension);
            let source_type2 = SourceType::default();

            // idempotency test
            let source_text3 = self.transform(path, &source_text2, source_type2);
            assert_eq!(source_text2, source_text3, "Idempotency test failed: {path:?}");

            fs::write(&new_path, source_text3).unwrap();
            fs::remove_file(path).unwrap();
            self.count += 1;
        }
    }

    fn transform(&self, path: &Path, source_text: &str, source_type: SourceType) -> String {
        let allocator = Allocator::default();

        let parser_ret = Parser::new(&allocator, source_text, source_type).parse();
        assert!(parser_ret.errors.is_empty(), "Expect no parse errors: {path:?}");

        let trivias = parser_ret.trivias;
        let mut program = parser_ret.program;

        let options = TransformOptions::default();
        Transformer::new(&allocator, path, source_type, source_text, &trivias, options)
            .build(&mut program)
            .unwrap_or_else(|_| panic!("Expect no transform errors: {path:?}"));

        let source_name = path.file_name().unwrap().to_string_lossy();
        let options = CodegenOptions::default();
        Codegen::<false>::new(&source_name, source_text, options, Default::default())
            .build(&program)
            .source_text
    }
}
