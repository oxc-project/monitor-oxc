use anyhow::Result;

use monitor_oxc::{
    codegen::CodegenRunner, isolated_declarations::test_isolated_declarations,
    mangler::ManglerRunner, transform::TransformRunner, NodeModulesRunner,
};

fn main() -> Result<()> {
    let node_modules_runner = NodeModulesRunner::new();
    let result = run(&node_modules_runner);
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
