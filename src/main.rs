use std::{path::PathBuf, process::ExitCode};

use pico_args::Arguments;

use monitor_oxc::{
    codegen::CodegenRunner, compressor::CompressorRunner, isolated_declarations,
    mangler::ManglerRunner, remove_whitespace::RemoveWhitespaceRunner,
    transformer::TransformerRunner, NodeModulesRunner,
};

fn main() -> ExitCode {
    let mut args = Arguments::from_env();

    let command = args.subcommand().expect("subcommand");
    let task = command.as_deref().unwrap_or("default");
    let filter: Option<String> = args.opt_value_from_str("--filter").unwrap();

    if matches!(task, "id") {
        let path_to_vue = args.opt_free_from_str::<PathBuf>().unwrap();
        return isolated_declarations::test(path_to_vue);
    }

    let mut node_modules_runner = NodeModulesRunner::new(filter.as_deref());

    if matches!(task, "codegen" | "default") {
        node_modules_runner.add_case(Box::new(CodegenRunner));
    }

    if matches!(task, "compress" | "compressor" | "default") {
        node_modules_runner.add_case(Box::new(CompressorRunner));
    }

    if matches!(task, "transform" | "transformer" | "default") {
        node_modules_runner.add_case(Box::new(TransformerRunner));
    }

    if matches!(task, "mangle" | "mangler" | "default") {
        node_modules_runner.add_case(Box::new(ManglerRunner));
    }

    if matches!(task, "whitespace" | "default") {
        node_modules_runner.add_case(Box::new(RemoveWhitespaceRunner));
    }

    let result = node_modules_runner.run_all();

    if let Err(diagnostics) = result {
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
    } else {
        ExitCode::SUCCESS
    }
}
