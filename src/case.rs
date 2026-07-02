use std::{fs, panic::RefUnwindSafe};

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

// `Sync` is required because `&dyn Case` is shared across rayon worker threads
// in `NodeModulesRunner::run_case`.
pub trait Case: RefUnwindSafe + Sync {
    fn name(&self) -> &'static str;

    fn enable_runtime_test(&self) -> bool {
        true
    }

    fn run_test(&self, _source: &Source) -> bool {
        true
    }

    /// Run the test for a single file. On success, returns any notes to be
    /// printed by the runner (in deterministic file order); on failure, returns
    /// diagnostics. Notes are returned rather than printed directly so that
    /// parallel execution stays deterministic.
    fn test(&self, source: &Source) -> Result<Vec<String>, Vec<Diagnostic>> {
        if self.run_test(source) {
            let (source_text, notes) = self.idempotency_test(source)?;
            // Write js files for runtime test
            if source.source_type.is_javascript() {
                fs::write(&source.path, source_text).unwrap();
            }
            Ok(notes)
        } else {
            Ok(Vec::new())
        }
    }

    fn driver(&self) -> Driver;

    /// Returns the idempotent output text together with any notes to print.
    fn idempotency_test(&self, source: &Source) -> Result<(String, Vec<String>), Vec<Diagnostic>> {
        let Source { path, source_type, source_text } = source;
        let source_text2 = self.driver().run(path, source_text, *source_type)?;
        let source_text3 = self.driver().run(path, &source_text2, *source_type)?;
        if source_text2 != source_text3 {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: NodeModulesRunner::get_diff(&source_text2, &source_text3, false),
            }]);
        }
        Ok((source_text3, Vec::new()))
    }
}
