//! Conformance check between Rust `oxc_codegen` and the JavaScript `oxc-codegen` package.
//!
//! Rust parses each monitored source and sends its ESTree representation to one persistent Node
//! process. That process prints with `oxc-codegen`; Rust prints the equivalent normalized AST and
//! compares the generated source byte-for-byte.

use std::{
    io::{BufRead, BufReader, Read as _, Write as _},
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use oxc::{
    allocator::Allocator,
    ast::ast::{
        ArrowFunctionExpression, ExportAllDeclaration, ExportFromDeclaration, Function,
        ImportDeclaration, WithClause, WithClauseKeyword,
    },
    ast_visit::{VisitMut, walk_mut},
    codegen::{Codegen, CodegenOptions, CommentOptions},
    parser::{ParseOptions, Parser},
    span::SourceType,
    syntax::scope::ScopeFlags,
};
use rayon::prelude::*;

use crate::{Diagnostic, NodeModulesRunner, NodeModulesRunnerOptions, Source};

const CASE_NAME: &str = "Codegen Conformance";

pub struct CodegenConformanceRunner;

impl CodegenConformanceRunner {
    pub fn run(options: NodeModulesRunnerOptions) -> Result<(), Vec<Diagnostic>> {
        let node_modules_runner = NodeModulesRunner::new(options);
        let file_count = node_modules_runner.files.len();
        let worker_count = rayon::current_num_threads().min(file_count);
        println!("Running {CASE_NAME} across {worker_count} JS processes.");

        // Split the files into exactly one contiguous shard per Rayon worker. Every worker owns
        // one persistent JS process, avoiding per-file Node startup while printing and parsing in
        // parallel across all configured CPU threads.
        let diagnostics = (0..worker_count)
            .into_par_iter()
            .map(|worker| {
                let start = worker * file_count / worker_count;
                let end = (worker + 1) * file_count / worker_count;
                test_sources(&node_modules_runner.files[start..end])
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        println!("Ran {file_count} times.");

        if diagnostics.is_empty() { Ok(()) } else { Err(diagnostics) }
    }
}

fn test_sources(sources: &[Source]) -> Vec<Diagnostic> {
    let mut js_codegen = match JsCodegen::spawn() {
        Ok(js_codegen) => js_codegen,
        Err(message) => return vec![Diagnostic { case: CASE_NAME, path: PathBuf::new(), message }],
    };
    let mut diagnostics = Vec::new();

    for source in sources {
        match catch_unwind(AssertUnwindSafe(|| js_codegen.test(source))) {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                diagnostics.push(Diagnostic { case: CASE_NAME, path: source.path.clone(), message })
            }
            Err(panic) => diagnostics.push(Diagnostic {
                case: CASE_NAME,
                path: source.path.clone(),
                message: format!("Rust codegen panicked: {panic:?}"),
            }),
        }
    }

    if let Err(message) = js_codegen.finish() {
        diagnostics.push(Diagnostic { case: CASE_NAME, path: PathBuf::new(), message });
    }

    diagnostics
}

struct JsCodegen {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl JsCodegen {
    fn spawn() -> Result<Self, String> {
        let codegen_path = Path::new("../oxc/packages/codegen/dist/index.js");
        if !codegen_path.is_file() {
            return Err(format!(
                "the JS codegen build is missing at {}. Build it with `pnpm --dir ../oxc --filter oxc-codegen run build`",
                codegen_path.display()
            ));
        }

        let mut child = Command::new("node")
            .arg("src/codegen-conformance.ts")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("failed to start the JS code generator: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "could not open stdin for the JS code generator".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "could not open stdout for the JS code generator".to_string())?;

        Ok(Self { child, stdin, stdout: BufReader::new(stdout) })
    }

    fn test(&mut self, source: &Source) -> Result<(), String> {
        let source_type = source_type_for_codegen(source);
        let allocator = Allocator::default();
        let mut parsed = Parser::new(&allocator, &source.source_text, source_type)
            .with_options(ParseOptions {
                // Keep parenthesized nodes, exactly as the parser's public JavaScript API does.
                preserve_parens: true,
                parse_regular_expression: true,
                allow_return_outside_function: true,
                ..ParseOptions::default()
            })
            .parse();

        // The package test suite only compares ASTs which parsed cleanly. Oxc can recover from a
        // parse error, but comparing its recovered Rust AST with the package is not meaningful.
        if parsed.panicked || !parsed.diagnostics.is_empty() {
            return Ok(());
        }

        // ESTree deliberately omits a few details Oxc retains internally. Erase those details
        // before printing Rust so both generators receive precisely the same information.
        Normalize { preserve_parens: true }.visit_program(&mut parsed.program);

        let rust_output = Codegen::new()
            .with_options(CodegenOptions {
                // `oxc-codegen` receives an ESTree AST without comment data and does not print
                // comments, so comments must be disabled on the Rust side too.
                comments: CommentOptions::disabled(),
                ..CodegenOptions::default()
            })
            .build(&parsed.program)
            .code;
        let program = parsed.program.to_estree_json(source_type.is_typescript(), false);
        let js_output = self.print(&program, source_type.is_typescript(), source_type.is_jsx())?;

        if rust_output == js_output {
            return Ok(());
        }

        Err(format!(
            "Rust and JavaScript codegen outputs differ.\nDiff (`-` Rust, `+` JS):\n{}",
            NodeModulesRunner::get_diff(&js_output, &rust_output, false)
        ))
    }

    fn print(&mut self, program: &str, ts: bool, jsx: bool) -> Result<String, String> {
        let request = format!(r#"{{"program":{program},"ts":{ts},"jsx":{jsx}}}"#);
        // AST `raw` fields can contain physical line breaks, so requests cannot be separated by
        // newlines. Send an ASCII byte length followed by the exact UTF-8 JSON payload instead.
        writeln!(self.stdin, "{}", request.len()).map_err(|error| {
            format!("failed to send AST length to the JS code generator: {error}")
        })?;
        self.stdin
            .write_all(request.as_bytes())
            .map_err(|error| format!("failed to send AST to the JS code generator: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("failed to flush AST to the JS code generator: {error}"))?;

        let mut header = String::new();
        let read = self
            .stdout
            .read_line(&mut header)
            .map_err(|error| format!("failed to read from the JS code generator: {error}"))?;
        if read == 0 {
            return Err("the JS code generator closed its output unexpectedly".to_string());
        }

        let mut fields = header.split_whitespace();
        let status = fields
            .next()
            .ok_or_else(|| format!("invalid JS code generator response: {header:?}"))?;
        let length = fields
            .next()
            .ok_or_else(|| format!("invalid JS code generator response: {header:?}"))?
            .parse::<usize>()
            .map_err(|error| format!("invalid JS code generator response length: {error}"))?;
        if fields.next().is_some() {
            return Err(format!("invalid JS code generator response: {header:?}"));
        }

        let mut bytes = vec![0; length];
        self.stdout
            .read_exact(&mut bytes)
            .map_err(|error| format!("failed to read JS code generator output: {error}"))?;
        let output = String::from_utf8(bytes)
            .map_err(|error| format!("JS code generator returned non-UTF-8 output: {error}"))?;

        match status {
            "OK" => Ok(output),
            "ERROR" => Err(format!("the JS code generator failed:\n{output}")),
            _ => Err(format!("invalid JS code generator response: {header:?}")),
        }
    }

    fn finish(self) -> Result<(), String> {
        let Self { mut child, stdin, stdout } = self;
        drop(stdin);
        drop(stdout);
        let status = child
            .wait()
            .map_err(|error| format!("failed while waiting for the JS code generator: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("the JS code generator exited with {status}"))
        }
    }
}

fn source_type_for_codegen(source: &Source) -> SourceType {
    // Keep the existing monitor's permissive `.js` parsing behavior so this command covers the
    // same files as the Rust codegen job.
    if source.path.extension().is_some_and(|extension| extension == "js") {
        source.source_type.with_jsx(source.source_type.is_javascript()).with_unambiguous(true)
    } else {
        source.source_type
    }
}

/// Removes the information Oxc's Rust AST has which cannot survive ESTree serialization.
///
/// This is the same representation boundary normalized in `oxc-codegen`'s fixture conformance
/// addon. A mismatch after this pass is therefore a printer mismatch, rather than a difference in
/// what the two AST formats can express.
struct Normalize {
    preserve_parens: bool,
}

impl<'a> VisitMut<'a> for Normalize {
    fn visit_function(&mut self, function: &mut Function<'a>, flags: ScopeFlags) {
        if !self.preserve_parens {
            function.pife = false;
        }
        walk_mut::walk_function(self, function, flags);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &mut ArrowFunctionExpression<'a>) {
        if !self.preserve_parens {
            arrow.pife = false;
        }
        walk_mut::walk_arrow_function_expression(self, arrow);
    }

    fn visit_import_declaration(&mut self, declaration: &mut ImportDeclaration<'a>) {
        if declaration.specifiers.as_ref().is_some_and(|specifiers| specifiers.is_empty()) {
            declaration.specifiers = None;
        }
        normalize_with_clause(&mut declaration.with_clause);
        walk_mut::walk_import_declaration(self, declaration);
    }

    fn visit_export_from_declaration(&mut self, declaration: &mut ExportFromDeclaration<'a>) {
        normalize_with_clause(&mut declaration.with_clause);
        walk_mut::walk_export_from_declaration(self, declaration);
    }

    fn visit_export_all_declaration(&mut self, declaration: &mut ExportAllDeclaration<'a>) {
        normalize_with_clause(&mut declaration.with_clause);
        walk_mut::walk_export_all_declaration(self, declaration);
    }
}

fn normalize_with_clause(with_clause: &mut Option<oxc::allocator::Box<'_, WithClause<'_>>>) {
    if let Some(clause) = with_clause {
        clause.keyword = WithClauseKeyword::With;
        if let Some(first) = clause.with_entries.first() {
            // ESTree exposes entries but not the `WithClause` wrapper's opening-brace span.
            clause.span.start = first.span.start;
        } else {
            *with_clause = None;
        }
    }
}
