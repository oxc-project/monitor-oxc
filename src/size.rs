use std::{
    any::Any,
    fs,
    io::Write as _,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    process::ExitCode,
};

use brotli::enc::BrotliEncoderParams;
use flate2::{Compression, write::GzEncoder};
use oxc::span::SourceType;
use rayon::prelude::*;

use crate::{
    Case, Diagnostic, Driver, NodeModulesRunner, compressor::CompressorRunner,
    minifier::MinifierRunner,
};

/// Minify every JS file once with the full pipeline (compress + mangle +
/// whitespace) and write one row per file, in walk order:
///
/// `path \t input_bytes \t status \t output_bytes \t gzip_bytes \t brotli_bytes \t detail`
///
/// `status` is `OK`, `INVALID` (the output fails to re-parse), `ERR` (the
/// minifier reported diagnostics) or `PANIC`. The three size columns are `-`
/// for `ERR` and `PANIC`. Unlike the other cases this never writes into
/// `node_modules`.
///
/// The base binary compiles this file against an older oxc commit, so it only
/// uses oxc through `Driver` (and `SourceType::from_path` in `emit`).
pub fn run(runner: &NodeModulesRunner, out: &Path) -> ExitCode {
    let rows = runner
        .files
        .par_iter()
        .filter(|source| source.is_js_only())
        .map(|source| {
            let result = catch_unwind(AssertUnwindSafe(|| {
                MinifierRunner.driver().run(&source.path, &source.source_text, source.source_type)
            }));
            let (status, sizes, detail) = match result {
                // Re-parse through a plain `Driver` so the same source type,
                // parse options and diagnostic filters apply.
                Ok(Ok(code)) => {
                    let reparsed = catch_unwind(AssertUnwindSafe(|| {
                        Driver::default().run(&source.path, &code, source.source_type)
                    }));
                    let sizes =
                        format!("{}\t{}\t{}", code.len(), gzip_size(&code), brotli_size(&code));
                    match reparsed {
                        Ok(Ok(_)) => ("OK", sizes, String::new()),
                        Ok(Err(diagnostics)) => ("INVALID", sizes, first_diagnostic(&diagnostics)),
                        Err(payload) => ("INVALID", sizes, first_line(panic_message(&*payload))),
                    }
                }
                Ok(Err(diagnostics)) => {
                    ("ERR", "-\t-\t-".to_string(), first_diagnostic(&diagnostics))
                }
                Err(payload) => {
                    ("PANIC", "-\t-\t-".to_string(), first_line(panic_message(&*payload)))
                }
            };
            format!(
                "{}\t{}\t{status}\t{sizes}\t{detail}\n",
                source.path.display(),
                source.source_text.len()
            )
        })
        .collect::<Vec<_>>();
    fs::write(out, rows.concat()).unwrap();
    println!("Wrote {} rows to {}.", rows.len(), out.display());
    ExitCode::SUCCESS
}

/// For each path listed in `paths` (one per line, as written by `run`), write
/// readable outputs under `out` so a base and a head run can be diffed:
///
/// - `compress/<path>`: compressor output, pretty-printed and unmangled.
///   Most minifier changes read best here: mangled names shift whenever
///   anything changes, which buries the real edit.
/// - `minify/<path>`: the full minify output reprinted readably (mangled
///   names kept), for changes that only show up with mangling.
/// - `raw/<path>`: the full minify output exactly as measured, for changes in
///   minified printing itself.
///
/// A stage that fails or panics writes no file for that view.
pub fn emit(paths: &Path, out: &Path) -> ExitCode {
    let list = fs::read_to_string(paths).unwrap();
    let paths = list.lines().filter(|line| !line.is_empty()).map(Path::new).collect::<Vec<_>>();
    paths.par_iter().for_each(|&path| {
        let (Ok(source_type), Ok(source_text)) =
            (SourceType::from_path(path), fs::read_to_string(path))
        else {
            return;
        };
        let write = |view: &str, code: &str| {
            let dest = out.join(view).join(path);
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::write(dest, code).unwrap();
        };
        let run = |mut driver: Driver, text: &str| {
            catch_unwind(AssertUnwindSafe(|| driver.run(path, text, source_type))).ok()?.ok()
        };
        if let Some(code) = run(CompressorRunner.driver(), &source_text) {
            write("compress", &code);
        }
        if let Some(code) = run(MinifierRunner.driver(), &source_text) {
            write("raw", &code);
            if let Some(pretty) = run(Driver::default(), &code) {
                write("minify", &pretty);
            }
        }
    });
    println!("Emitted {} files to {}.", paths.len(), out.display());
    ExitCode::SUCCESS
}

/// Gzip at maximum compression, as oxc's `tasks/minsize` measures it.
fn gzip_size(code: &str) -> usize {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(code.as_bytes()).unwrap();
    encoder.finish().unwrap().len()
}

/// Brotli at quality 5 (4 MiB window), close to what CDNs use when they
/// compress on the fly. Quality 11, the precompressed-asset setting, is ~60x
/// slower on this corpus and would turn a 20 s run into 5 minutes; the base vs
/// head change, which is what the report shows, moves the same way.
fn brotli_size(code: &str) -> usize {
    let params = BrotliEncoderParams { quality: 5, ..BrotliEncoderParams::default() };
    let mut compressed = Vec::new();
    brotli::BrotliCompress(&mut code.as_bytes(), &mut compressed, &params).unwrap();
    compressed.len()
}

fn first_diagnostic(diagnostics: &[Diagnostic]) -> String {
    diagnostics.first().map_or_else(String::new, |d| first_line(&d.message))
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic")
}

/// First non-empty line, tab-free and capped at 200 chars, so it fits in a
/// TSV cell.
fn first_line(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
    line.chars().take(200).map(|c| if c == '\t' { ' ' } else { c }).collect()
}
