use std::{fs, path::Path};

use oxc::span::SourceType;

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

pub struct TransformRunner;

impl TransformRunner {
    pub fn run(runner: &NodeModulesRunner) -> Result<(), Vec<Diagnostic>> {
        println!("Running Transformer");
        runner.run(Self::test)
    }

    fn test(source: &Source) -> Result<(), Diagnostic> {
        let source_text =
            NodeModulesRunner::idempotency_test("transform", source, Self::transform)?;
        // Write files for runtime test
        let new_extension = source.path.extension().unwrap().to_string_lossy().replace('t', "j");
        let new_path = source.path.with_extension(new_extension);
        fs::write(new_path, source_text).unwrap();
        Ok(())
    }

    pub fn transform(
        source_path: &Path,
        source_text: &str,
        source_type: SourceType,
    ) -> Result<String, Diagnostic> {
        Driver::default().with_transform().run(source_path, source_text, source_type)
    }
}
