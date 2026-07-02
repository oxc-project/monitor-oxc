use std::{
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
    thread,
};

use oxc::allocator::Allocator;
use oxc_formatter::JsFormatOptions;

use crate::{Case, Diagnostic, Driver, NodeModulesRunner, Source};

pub struct FormatterRunner;

impl Case for FormatterRunner {
    fn name(&self) -> &'static str {
        "Formatter"
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn driver(&self) -> Driver {
        unreachable!()
    }

    fn idempotency_test(&self, source: &Source) -> Result<(String, Vec<String>), Vec<Diagnostic>> {
        let Source { path, source_type, source_text } = source;

        let allocator = Allocator::new();

        let source_text2 = match oxc_formatter::format(
            &allocator,
            source_text,
            *source_type,
            JsFormatOptions::default(),
            None,
        ) {
            Ok(formatted) => formatted.print().unwrap().into_code(),
            Err(error) => {
                // oxc does not support Flow syntax, so Flow files can't be
                // formatted. Skip them instead of reporting a failure (the
                // transformer driver filters the same diagnostic).
                if error.message.starts_with("Flow is not supported") {
                    return Ok((source_text.clone(), Vec::new()));
                }
                return Err(vec![Diagnostic {
                    case: self.name(),
                    path: path.clone(),
                    message: format!("Parse error in original source: {error:?}"),
                }]);
            }
        };

        let allocator2 = Allocator::new();
        let source_text3 = match oxc_formatter::format(
            &allocator2,
            &source_text2,
            *source_type,
            JsFormatOptions::default(),
            None,
        ) {
            Ok(formatted) => formatted.print().unwrap().into_code(),
            Err(error) => {
                return Err(vec![Diagnostic {
                    case: self.name(),
                    path: path.clone(),
                    message: format!("Parse error after formatting: {error:?}"),
                }]);
            }
        };

        if source_text2 != source_text3 {
            return match classify_against_prettier(path, &source_text2, &source_text3) {
                // oxfmt agrees with Prettier on one pass: a real idempotency bug, but
                // not a Prettier-conformance gap. Report it as a warning, don't fail.
                Verdict::Matched(message) => {
                    // Return the note instead of printing it here, so the runner can
                    // emit it in deterministic order after the parallel phase.
                    let note = format!("{}\n{}\n{message}\n", self.name(), path.to_string_lossy());
                    Ok((source_text3, vec![note]))
                }
                // oxfmt diverges from Prettier (or Prettier could not classify it): fail.
                Verdict::Mismatch(message) => {
                    Err(vec![Diagnostic { case: self.name(), path: path.clone(), message }])
                }
            };
        }

        Ok((source_text3, Vec::new()))
    }
}

/// Outcome of comparing an idempotency failure against Prettier.
enum Verdict {
    /// One of oxfmt's passes equals Prettier — reported but not a failure.
    Matched(String),
    /// Neither pass equals Prettier (or Prettier was unavailable) — a failure.
    Mismatch(String),
}

/// Classify an idempotency failure against Prettier.
///
/// Prettier is idempotent, so for any file where oxfmt is *not* (`pass1 != pass2`)
/// at most one of the two oxfmt passes can equal Prettier's output. We format the
/// *already-formatted* `pass1` with Prettier (Prettier is its own fixed point, so
/// this is a stable reference) and report:
///
/// - `MATCHED`:  one of oxfmt's passes equals Prettier — oxfmt is Prettier-correct
///   on that pass but not idempotent across re-formats.
/// - `MISMATCH`: neither pass equals Prettier — oxfmt genuinely diverges from
///   Prettier; the oxfmt-vs-Prettier diff is the actionable one.
fn classify_against_prettier(path: &Path, pass1: &str, pass2: &str) -> Verdict {
    let idempotency_diff = NodeModulesRunner::get_diff(pass1, pass2, false);

    match run_prettier(path, pass1) {
        Ok(prettier) => {
            let p = prettier.trim_end_matches('\n');
            let a = pass1.trim_end_matches('\n');
            let b = pass2.trim_end_matches('\n');
            if a == p {
                Verdict::Matched(format!(
                    "[Prettier: MATCHED] oxfmt pass-1 matches Prettier; oxfmt is not idempotent (pass-2 drifts away).\nidempotency diff (`+` pass-1, `-` pass-2):\n{idempotency_diff}"
                ))
            } else if b == p {
                Verdict::Matched(format!(
                    "[Prettier: MATCHED] oxfmt pass-2 matches Prettier; oxfmt pass-1 drifts (not idempotent).\nidempotency diff (`+` pass-1, `-` pass-2):\n{idempotency_diff}"
                ))
            } else {
                Verdict::Mismatch(format!(
                    "[Prettier: MISMATCH] neither oxfmt pass matches Prettier.\ndiff (`-` oxfmt pass-1, `+` Prettier):\n{}",
                    NodeModulesRunner::get_diff(&prettier, pass1, false)
                ))
            }
        }
        // Without Prettier we can't tell a real divergence from instability, so fail
        // to stay safe rather than silently passing a possible mismatch.
        Err(err) => Verdict::Mismatch(format!(
            "[Prettier: unavailable] {err}\nidempotency diff (`+` pass-1, `-` pass-2):\n{idempotency_diff}"
        )),
    }
}

/// Format `source` with Prettier using options that match oxfmt's
/// `JsFormatOptions::default()`.
///
/// The only oxfmt default that differs from Prettier's own defaults is the print
/// width (oxfmt: 100, Prettier: 80), so we set `--print-width 100`; every other
/// oxfmt default already equals a Prettier default. `--no-config`/`--no-editorconfig`
/// keep stray config files from skewing the comparison.
///
/// Prettier silently ignores any path under `node_modules` (even with
/// `--with-node-modules` when fed via `--stdin-filepath`), echoing stdin back
/// unchanged — which would make every file look like it matches. Since all our
/// inputs live under `node_modules`, we hand Prettier a synthetic filename that
/// keeps the real extension (so the parser is still inferred correctly: `.ts` →
/// typescript, `.tsx`/`.jsx` → with JSX, `.mts`/`.cts` → typescript, etc.) but is
/// not under `node_modules`.
fn run_prettier(path: &Path, source: &str) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("js");
    let stdin_filepath = format!("stdin.{ext}");
    let mut child = Command::new("node_modules/.bin/prettier")
        .arg("--stdin-filepath")
        .arg(&stdin_filepath)
        .arg("--no-config")
        .arg("--no-editorconfig")
        .arg("--print-width")
        .arg("100")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn prettier: {e}"))?;

    // Write stdin from a separate thread to avoid a pipe deadlock on large files
    // (Prettier may start writing stdout before we finish writing stdin).
    let mut stdin = child.stdin.take().unwrap();
    let owned = source.to_owned();
    let writer = thread::spawn(move || {
        let _ = stdin.write_all(owned.as_bytes());
        // `stdin` is dropped here, closing the pipe so Prettier sees EOF.
    });

    let output = child.wait_with_output().map_err(|e| format!("failed to run prettier: {e}"))?;
    let _ = writer.join();

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
