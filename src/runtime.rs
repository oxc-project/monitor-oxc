use std::{fs, panic::catch_unwind, path::Path, process::ExitCode};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use serde::Deserialize;
use walkdir::WalkDir;

use oxc::{minifier::CompressOptions, span::SourceType};

use crate::{Diagnostic, Driver, transformer};

/// Entry in `runtime-repos.json`. Only the fields the minifier needs are
/// deserialized; `repo` / `sha` / `install` / `test` / `notes` are consumed
/// by CI and `scripts/runtime-test.mjs`.
#[derive(Deserialize)]
struct RepoConfig {
    name: String,
    sources: Vec<String>,
    #[serde(default)]
    ignore: Vec<String>,
}

/// Minify (transform + compress + mangle + remove whitespace) every file in
/// `dir` matched by the named config entry, rewriting each file in place and
/// keeping its extension, so JS emitted into a `.ts` file still flows through
/// the repo's own test transpilation untouched.
pub fn run(config_path: &Path, name: &str, dir: &Path) -> ExitCode {
    let config = load_config(config_path, name);
    let sources = build_globset(&config.sources);
    let ignore = build_globset(&config.ignore);

    let paths = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let rel = path.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
            if rel.contains("node_modules") || !sources.is_match(&rel) || ignore.is_match(&rel) {
                return None;
            }
            let source_type = SourceType::from_path(path).ok()?;
            if source_type.is_typescript_definition() {
                return None;
            }
            Some((path.to_path_buf(), source_type))
        })
        .collect::<Vec<_>>();

    if paths.is_empty() {
        println!("No files matched the source globs for {name} in {}.", dir.display());
        return ExitCode::FAILURE;
    }

    let diagnostics = paths
        .par_iter()
        .filter_map(|(path, source_type)| {
            let source_text =
                fs::read_to_string(path).unwrap_or_else(|e| panic!("{e:?}\n{}", path.display()));
            match catch_unwind(|| minify_driver().run(path, &source_text, *source_type)) {
                Ok(Ok(minified)) => {
                    fs::write(path, minified).unwrap();
                    None
                }
                Ok(Err(diagnostics)) => Some(diagnostics),
                Err(err) => Some(vec![Diagnostic {
                    case: "RuntimeMinify",
                    path: path.clone(),
                    message: format!("{err:?}"),
                }]),
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    println!("Processed {} files.", paths.len());
    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        for diagnostic in &diagnostics {
            println!(
                "{}\n{}\n{}",
                diagnostic.case,
                diagnostic.path.to_string_lossy(),
                diagnostic.message
            );
        }
        println!("{} Failed.", diagnostics.len());
        ExitCode::FAILURE
    }
}

fn load_config(path: &Path, name: &str) -> RepoConfig {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut repos: Vec<RepoConfig> = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
    let index = repos
        .iter()
        .position(|repo| repo.name == name)
        .unwrap_or_else(|| panic!("no repo named `{name}` in {}", path.display()));
    repos.swap_remove(index)
}

fn build_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).unwrap_or_else(|e| panic!("bad glob `{pattern}`: {e}")));
    }
    builder.build().unwrap()
}

fn minify_driver() -> Driver {
    Driver {
        transform: Some(transformer::transform_options()),
        compress: Some(CompressOptions::default()),
        mangle: true,
        remove_whitespace: true,
        ..Driver::default()
    }
}
