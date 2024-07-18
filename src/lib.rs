#![allow(clippy::new_without_default)]

pub mod codegen;
pub mod isolated_declarations;
pub mod mangler;
pub mod transformer;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use console::Style;
use similar::{ChangeTag, TextDiff};
use walkdir::WalkDir;

use oxc::span::SourceType;

pub struct Diagnostic {
    pub case: &'static str,
    pub path: PathBuf,
    pub message: String,
}

pub struct Source {
    pub path: PathBuf,
    pub source_type: SourceType,
    pub source_text: String,
}

pub struct NodeModulesRunner {
    pub files: Vec<Source>,
}

const PATH_IGNORES: &[&str] =
    &["node_modules/.pnpm/node-domexception@1.0.0/node_modules/node-domexception/.history"];

impl NodeModulesRunner {
    pub fn new() -> Self {
        let mut files = vec![];
        for entry in WalkDir::new("node_modules/.pnpm") {
            let dir_entry = entry.unwrap();
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            if PATH_IGNORES.iter().any(|p| path.starts_with(p)) {
                continue;
            }
            let Ok(source_type) = SourceType::from_path(path) else {
                continue;
            };
            if source_type.is_typescript_definition() {
                continue;
            }
            let source_text = fs::read_to_string(path).unwrap();
            if source_text.starts_with("// @flow") {
                continue;
            }
            files.push(Source { path: path.to_path_buf(), source_type, source_text });
        }
        println!("Collected {} files.", files.len());
        Self { files }
    }

    pub fn recover(self) {
        for source in self.files {
            fs::write(source.path, source.source_text).unwrap();
        }
    }

    pub fn run<F>(&self, f: F) -> Result<(), Vec<Diagnostic>>
    where
        F: Fn(&Source) -> Result<(), Diagnostic>,
    {
        let diagnostics = self
            .files
            .iter()
            .filter_map(|source| if let Err(d) = f(source) { Some(d) } else { None })
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        Self::runtime_test()
    }

    fn runtime_test() -> Result<(), Vec<Diagnostic>> {
        println!("pnpm test");
        if let Err(err) = Command::new("pnpm").arg("test").status() {
            return Err(vec![Diagnostic {
                case: "pnpm test",
                path: PathBuf::new(),
                message: err.to_string(),
            }]);
        }
        Ok(())
    }

    pub fn idempotency_test<F>(
        case: &'static str,
        source: &Source,
        f: F,
    ) -> Result<String, Diagnostic>
    where
        F: Fn(&Path, &str, SourceType) -> Result<String, Diagnostic>,
    {
        let Source { path, source_type, source_text } = source;
        let source_text2 = f(path, source_text, *source_type)?;
        let source_text3 = f(path, &source_text2, *source_type)?;
        if source_text2 != source_text3 {
            return Err(Diagnostic {
                case,
                path: path.clone(),
                message: NodeModulesRunner::print_diff(&source_text2, &source_text3),
            });
        }
        Ok(source_text3)
    }

    pub fn print_diff(origin_string: &str, expected_string: &str) -> String {
        let diff = TextDiff::from_lines(expected_string, origin_string);
        let mut output = String::new();
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal => continue, // (" ", Style::new()),
            };
            output.push_str(&format!("{}{}", style.apply_to(sign).bold(), style.apply_to(change)));
        }
        output
    }
}
