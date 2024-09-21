use std::fs;

use crate::{driver::default_transformer_options, Case, Diagnostic, Driver, Source};

pub struct TransformerRunner;

impl Case for TransformerRunner {
    fn name(&self) -> &'static str {
        "Transformer"
    }

    fn test(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        if self.run_test(source.source_type) {
            let path = &source.path;
            let source_text = self.idempotency_test(source)?;
            // Write js files for runtime test
            let new_extension = path.extension().unwrap().to_string_lossy().replace('t', "j");
            let new_path = path.with_extension(new_extension);
            fs::write(new_path, source_text).unwrap();
        }
        Ok(())
    }

    fn driver(&self) -> Driver {
        Driver { transform: Some(default_transformer_options()), ..Driver::default() }
    }
}
