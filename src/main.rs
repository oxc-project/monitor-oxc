use std::{path::PathBuf, process::ExitCode};

use pico_args::Arguments;

use monitor_oxc::{
    Diagnostic, NodeModulesRunner, NodeModulesRunnerOptions, codegen::CodegenRunner,
    codegen_conformance::CodegenConformanceRunner,
    compressor::CompressorRunner, dce::DceRunner, formatter::FormatterRunner,
    formatter_dcr::FormatterDCRRunner, isolated_declarations, mangler::ManglerRunner,
    minifier::MinifierRunner, remove_whitespace::RemoveWhitespaceRunner,
    transformer::TransformerRunner, NodeModulesRunner, NodeModulesRunnerOptions,
};

fn main() -> ExitCode {
    // Rayon workers default to 2 MiB stacks, which deeply nested expressions
    // in node_modules overflow (an abort, not a catchable panic). Match the
    // main thread's 8 MiB, as oxc's coverage runner does.
    rayon::ThreadPoolBuilder::new().stack_size(8 * 1024 * 1024).build_global().unwrap();

    let mut args = Arguments::from_env();

    let options = NodeModulesRunnerOptions {
        filter: args.opt_value_from_str("--filter").unwrap(),
        no_restore: args.contains("--no-restore"),
    };

    let task = args.free_from_str().unwrap_or_else(|_| "default".to_string());
    let task = task.as_str();

    println!("Task: {task}");

    if matches!(task, "id") {
        let path_to_vue = args.opt_free_from_str::<PathBuf>().unwrap();
        return isolated_declarations::test(path_to_vue);
    }

    if matches!(task, "runtime-minify") {
        let config: PathBuf = args
            .opt_value_from_str("--config")
            .unwrap()
            .unwrap_or_else(|| PathBuf::from("runtime-repos.json"));
        let name: String = args.value_from_str("--name").unwrap();
        let dir: PathBuf = args.value_from_str("--dir").unwrap();
        return monitor_oxc::runtime::run(&config, &name, &dir);
    }

    println!("Options: {options:?}");

    if matches!(task, "codegen_conformance" | "conformance") {
        return match CodegenConformanceRunner::run(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(diagnostics) => {
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
        };
    }

    let mut node_modules_runner = NodeModulesRunner::new(options);

    if matches!(task, "codegen" | "default") {
        node_modules_runner.add_case(Box::new(CodegenRunner));
    }

    if matches!(task, "transform" | "transformer" | "default") {
        node_modules_runner.add_case(Box::new(TransformerRunner));
    }

    if matches!(task, "dce" | "default") {
        node_modules_runner.add_case(Box::new(DceRunner));
    }

    if matches!(task, "compress" | "compressor" | "default") {
        node_modules_runner.add_case(Box::new(CompressorRunner));
    }

    if matches!(task, "mangle" | "mangler" | "default") {
        node_modules_runner.add_case(Box::new(ManglerRunner));
    }

    if matches!(task, "whitespace" | "default") {
        node_modules_runner.add_case(Box::new(RemoveWhitespaceRunner));
    }

    if matches!(task, "minifier" | "minify" | "default") {
        node_modules_runner.add_case(Box::new(MinifierRunner));
    }

    if matches!(task, "formatter" | "default") {
        node_modules_runner.add_case(Box::new(FormatterRunner));
    }
    if matches!(task, "formatter_dcr" | "default") {
        node_modules_runner.add_case(Box::new(FormatterDCRRunner));
    }

    let result = node_modules_runner.run_all();

    match result {
        Err(diagnostics) => Diagnostic::report(&diagnostics),
        Ok(()) => ExitCode::SUCCESS,
    }
}
