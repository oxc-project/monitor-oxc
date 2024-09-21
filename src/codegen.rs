use crate::{Case, Driver};

pub struct CodegenRunner;

impl Case for CodegenRunner {
    fn name(&self) -> &'static str {
        "Codegen"
    }

    fn driver(&self) -> Driver {
        Driver::default()
    }
}
