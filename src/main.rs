use std::process::ExitCode;

use pico_args::Arguments;

use monitor_oxc::{
    codegen::CodegenRunner, isolated_declarations::test_isolated_declarations,
    mangler::ManglerRunner, transformer::TransformRunner, Diagnostic, NodeModulesRunner,
};

fn main() -> ExitCode {
    let mut args = Arguments::from_env();

    let command = args.subcommand().expect("subcommand");
    let task = command.as_deref().unwrap_or("default");

    let node_modules_runner = NodeModulesRunner::new();

    let result = match task {
        "codegen" => CodegenRunner::run(&node_modules_runner),
        "transformer" => TransformRunner::run(&node_modules_runner),
        "mangler" => ManglerRunner::run(&node_modules_runner),
        "id" => {
            test_isolated_declarations();
            Ok(())
        }
        _ => run(&node_modules_runner),
    };

    let exit_code = if let Err(diagnostics) = result {
        for diagnostic in &diagnostics {
            println!("{}\n{:?}\n{}", diagnostic.case, diagnostic.path, diagnostic.message);
        }
        println!("{} Failed.", diagnostics.len());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };

    node_modules_runner.recover();

    exit_code
}

fn run(node_modules_runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
    CodegenRunner::run(node_modules_runner)?;
    TransformRunner::run(node_modules_runner)?;
    ManglerRunner::run(node_modules_runner)?;
    // test_isolated_declarations();
    Ok(())
}
