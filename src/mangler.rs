use std::{fs, path::Path};

use oxc::span::SourceType;

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

pub struct ManglerRunner;

impl ManglerRunner {
    pub fn run(runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Mangler");
        runner.run(Self::test)
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let source_text = NodeModulesRunner::idempotency_test("mangler", source, Self::mangle)?;
        // Write js files for runtime test
        if source.source_type.is_javascript() {
            fs::write(&source.path, source_text).unwrap();
        }
        Ok(())
    }

    fn mangle(
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        Driver::default().with_mangle().run(source_path, source_text, source_type)
    }
}
