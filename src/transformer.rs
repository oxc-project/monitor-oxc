use std::fs;

use oxc::transformer::TransformOptions;

use crate::{Case, Diagnostic, Driver, Source};

pub struct TransformerRunner;

impl Case for TransformerRunner {
    fn name(&self) -> &'static str {
        "Transformer"
    }

    fn test(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        let path = &source.path;
        let source_text = self.idempotency_test(source)?;
        // Write js files for runtime test
        let new_extension = path.extension().unwrap().to_string_lossy().replace('t', "j");
        let new_path = path.with_extension(new_extension);
        fs::write(new_path, source_text).unwrap();
        Ok(())
    }

    fn driver(&self) -> Driver {
        let mut options = TransformOptions::enable_all();
        // Turns off the refresh plugin because it is never idempotent
        options.jsx.refresh = None;
        // Enables `only_remove_type_imports` avoiding removing all unused imports
        options.typescript.only_remove_type_imports = true;

        // These two injects helper in esm format, which breaks cjs files.
        options.env.es2018.async_generator_functions = false;
        options.env.es2017.async_to_generator = false;

        Driver { transform: Some(options), ..Driver::default() }
    }
}
