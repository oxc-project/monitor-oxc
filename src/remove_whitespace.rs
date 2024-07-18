use std::{fs, path::Path};

use oxc::span::SourceType;

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

pub struct RemoveWhitespaceRunner;

impl RemoveWhitespaceRunner {
    pub fn run(runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Remove Whitespace");
        runner.run(Self::test)
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let source_text = NodeModulesRunner::idempotency_test("codegen", source, Self::codegen)?;
        // Write js files for runtime test
        if source.source_type.is_javascript() {
            fs::write(&source.path, source_text).unwrap();
        }
        Ok(())
    }

    fn codegen(
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        Driver::default().with_remove_whitespace().run(source_path, source_text, source_type)
    }
}
