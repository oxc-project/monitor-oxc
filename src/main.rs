use monitor_oxc::{codegen::CodegenRunner, isolated_declarations::test_isolated_declarations};

fn main() {
    CodegenRunner.run();
    test_isolated_declarations();
}
