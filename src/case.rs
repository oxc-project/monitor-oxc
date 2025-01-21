use std::{fs, panic::RefUnwindSafe};

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

pub trait Case: RefUnwindSafe {
    fn name(&self) -> &'static str;

    fn enable_runtime_test(&self) -> bool {
        true
    }

    fn run_test(&self, _source: &Source) -> bool {
        true
    }

    fn test(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        if self.run_test(source) {
            let source_text = self.idempotency_test(source)?;
            // Write js files for runtime test
            if source.source_type.is_javascript() {
                fs::write(&source.path, source_text).unwrap();
            }
        }
        Ok(())
    }

    fn driver(&self) -> Driver;

    fn idempotency_test(&self, source: &Source) -> Result<String, Vec<Diagnostic>> {
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
        Ok(source_text3)
    }
}
