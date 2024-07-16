use monitor_oxc::{
    codegen::CodegenRunner, isolated_declarations::test_isolated_declarations,
    transform::TransformRunner,
};

fn main() {
    CodegenRunner::new().run();
    TransformRunner::new().run();
    test_isolated_declarations();
}
