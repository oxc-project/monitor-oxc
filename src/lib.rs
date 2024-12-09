pub mod codegen;
pub mod compressor;
pub mod dce;
pub mod isolated_declarations;
pub mod mangler;
pub mod remove_whitespace;
pub mod transformer;

mod case;
mod driver;

use std::{fs, panic::catch_unwind, path::PathBuf, process::Command};

use console::Style;
use similar::{ChangeTag, TextDiff};
use walkdir::WalkDir;

use oxc::span::SourceType;

use case::Case;
use driver::Driver;

const PATH_IGNORES: &[&str] = &[
    "node_modules/node-domexception/.history",
    // intentional parse errors
    "node_modules/thread-stream/test/syntax-error.mjs",
    "node_modules/pino/test/fixtures/syntax-error-esm.mjs",
    "node_modules/charenc/README.js",
    // broken types
    "node_modules/immer/src/types/types-internal.ts",
    "node_modules/fbjs/flow/lib/dev.js",
    "node_modules/fast-copy/flow-typed/fast-copy.js",
    // template files
    ".vitepress",
    "node_modules/next/dist/esm/build/templates/app-page.js",
    "node_modules/karma/config.tpl.ts",
    "node_modules/karma/config.tpl.js",
    // with statement
    "node_modules/es-shim-unscopables/test/with.js",
    // gzipped file
    ".min.gzip.js",
    // bash file
    "node_modules/.bin/sha.js",
    // not strict mode
    "node_modules/esprima/test/test.js",
    "node_modules/memjs/test/client_test.js",
    "node_modules/lunr/test/pipeline_test.js",
    // broken syntax
    "node_modules/eslint-plugin-flowtype/dist/bin/checkDocs.js",
    "node_modules/limiter/test/tokenbucket-test.js",
    "node_modules/fast-json-patch/webpack.config.js",
    "node_modules/cardinal/test/fixtures/foo-with-errors.js",
    "node_modules/js-beautify/js/src/unpackers/p_a_c_k_e_r_unpacker.js",
    "node_modules/comment-parser/tests/e2e/examples.js",
    "node_modules/react-native/Libraries/Utilities/__mocks__/BackHandler.js",
    "node_modules/sass/sass.dart.js",
];

#[derive(Debug)]
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

impl Source {
    pub fn is_js_only(&self) -> bool {
        self.source_type.is_javascript()
            && !self.path.extension().is_some_and(|ext| ext.to_str().unwrap().contains('x'))
    }
}

pub struct NodeModulesRunner {
    pub files: Vec<Source>,
    pub cases: Vec<Box<dyn Case>>,
}

impl NodeModulesRunner {
    pub fn new(filter: Option<&str>) -> Self {
        let mut files = vec![];
        for entry in WalkDir::new("node_modules/.pnpm") {
            let dir_entry = entry.unwrap();
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(filter) = filter.as_ref() {
                let path = path.to_string_lossy();
                if path.contains(filter) {
                    println!("Filtered {path}");
                } else {
                    continue;
                }
            }
            let Ok(source_type) = SourceType::from_path(path) else {
                continue;
            };
            if PATH_IGNORES.iter().any(|p| path.to_string_lossy().contains(p)) {
                continue;
            }
            if source_type.is_typescript_definition() {
                continue;
            }
            let source_text =
                fs::read_to_string(path).unwrap_or_else(|e| panic!("{e:?}\n{path:?}"));
            files.push(Source { path: path.to_path_buf(), source_type, source_text });
        }
        println!("Collected {} files.", files.len());
        Self { files, cases: vec![] }
    }

    pub fn add_case(&mut self, case: Box<dyn Case>) {
        self.cases.push(case);
    }

    pub fn run_all(&self) -> Result<(), Vec<Diagnostic>> {
        for case in &self.cases {
            self.run_case(&**case)?;
        }
        Ok(())
    }

    fn run_case(&self, case: &dyn Case) -> Result<(), Vec<Diagnostic>> {
        println!("Running {}.", case.name());
        let results = self
            .files
            .iter()
            .map(|source| {
                catch_unwind(|| case.test(source)).map_err(|err| {
                    vec![Diagnostic {
                        case: case.name(),
                        path: source.path.clone(),
                        message: format!("{err:?}"),
                    }]
                })
            })
            .flatten()
            .collect::<Vec<_>>();
        println!("Ran {} times.", results.len());
        let diagnostics = results
            .into_iter()
            .filter_map(|source| if let Err(d) = source { Some(d) } else { None })
            .flatten()
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        let result = Self::runtime_test(case.name());
        self.restore_files(case.name());
        result
    }

    fn restore_files(&self, name: &str) {
        println!("Restoring files for {name}");
        for source in &self.files {
            fs::write(&source.path, &source.source_text).unwrap();
        }
    }

    fn runtime_test(name: &str) -> Result<(), Vec<Diagnostic>> {
        println!("pnpm test");
        match Command::new("pnpm").arg("test").status() {
            Ok(exit_status) => {
                if exit_status.code().is_some_and(|code| code == 0) {
                    Ok(())
                } else {
                    Err(vec![Diagnostic {
                        case: "pnpm test",
                        path: PathBuf::new(),
                        message: format!("pnpm failed for {name}"),
                    }])
                }
            }
            Err(err) => Err(vec![Diagnostic {
                case: "pnpm test",
                path: PathBuf::new(),
                message: err.to_string(),
            }]),
        }
    }

    #[must_use]
    pub fn get_diff(origin_string: &str, expected_string: &str, print_all: bool) -> String {
        let diff = TextDiff::from_lines(expected_string, origin_string);
        let mut output = String::new();
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal if print_all => (" ", Style::new()),
                ChangeTag::Equal => continue,
            };
            output.push_str(&format!("{}{}", style.apply_to(sign).bold(), style.apply_to(change)));
        }
        output
    }
}
