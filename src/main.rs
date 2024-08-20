use std::process::ExitCode;

use pico_args::Arguments;

use monitor_oxc::{
    codegen::CodegenRunner, compressor::CompressorRunner, mangler::ManglerRunner,
    remove_whitespace::RemoveWhitespaceRunner, transformer::TransformerRunner, NodeModulesRunner,
};

fn main() -> ExitCode {
    let mut args = Arguments::from_env();

    let command = args.subcommand().expect("subcommand");
    let task = command.as_deref().unwrap_or("default");

    let mut node_modules_runner = NodeModulesRunner::new();

    if matches!(task, "codegen" | "default") {
        node_modules_runner.add_case(Box::new(CodegenRunner));
    }

    if matches!(task, "compress" | "default") {
        node_modules_runner.add_case(Box::new(CompressorRunner));
    }

    if matches!(task, "transform" | "default") {
        node_modules_runner.add_case(Box::new(TransformerRunner));
    }

    if matches!(task, "mangler" | "default") {
        node_modules_runner.add_case(Box::new(ManglerRunner));
    }

    if matches!(task, "whitespace" | "default") {
        node_modules_runner.add_case(Box::new(RemoveWhitespaceRunner));
    }

    // if matches!(task, "id" | "default") {
    // isolated_declarations::test_isolated_declarations::test_isolated_declarations();
    // }

    let result = node_modules_runner.run_all();

    if let Err(diagnostics) = result {
        for diagnostic in &diagnostics {
            println!("{}\n{:?}\n{}", diagnostic.case, diagnostic.path, diagnostic.message);
        }
        println!("{} Failed.", diagnostics.len());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
