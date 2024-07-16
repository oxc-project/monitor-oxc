pub mod codegen;
pub mod isolated_declarations;
pub mod transform;

use std::{fs, os::unix::process::CommandExt, path::PathBuf, process::Command};

use console::Style;
use similar::{ChangeTag, TextDiff};
use walkdir::WalkDir;

use oxc::span::SourceType;

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
        for entry in WalkDir::new("node_modules/.pnpm").into_iter() {
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
            let source_text = fs::read_to_string(path).unwrap();
            files.push(Source { path: path.to_path_buf(), source_type, source_text });
        }
        println!("Collected {} files.", files.len());
        Self { files }
    }

    pub fn run_runtime_test() {
        Command::new("pnpm").arg("test").exec();
    }

    pub fn print_diff(origin_string: &str, expected_string: &str) {
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
}
