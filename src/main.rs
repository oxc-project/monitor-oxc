use anyhow::Result;
use pico_args::Arguments;

use monitor_oxc::{
    codegen::CodegenRunner, isolated_declarations::test_isolated_declarations,
    mangler::ManglerRunner, transformer::TransformRunner, NodeModulesRunner,
};

fn main() -> Result<()> {
    let mut args = Arguments::from_env();

    let command = args.subcommand().expect("subcommand");
    let task = command.as_deref().unwrap_or("default");

    let node_modules_runner = NodeModulesRunner::new();

    let result = match task {
        "codegen" => CodegenRunner.run(&node_modules_runner),
        "transformer" => TransformRunner.run(&node_modules_runner),
        "mangler" => ManglerRunner.run(&node_modules_runner),
        "id" => {
            test_isolated_declarations();
            Ok(())
        }
        _ => run(&node_modules_runner),
    };

    if result.is_err() {
        node_modules_runner.recover();
    }
    result
}

fn run(node_modules_runner: &NodeModulesRunner) -> Result<()> {
    CodegenRunner.run(node_modules_runner)?;
    TransformRunner.run(node_modules_runner)?;
    ManglerRunner.run(node_modules_runner)?;
    test_isolated_declarations();
    Ok(())
}
