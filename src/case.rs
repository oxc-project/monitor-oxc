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

    /// Run the test for a single file. On success, returns an optional note to
    /// be printed by the runner (in deterministic file order); on failure,
    /// returns diagnostics. The note is returned rather than printed directly so
    /// that parallel execution stays deterministic.
    fn test(&self, source: &Source) -> Result<Option<String>, Vec<Diagnostic>> {
        if self.run_test(source) {
            let (source_text, note) = self.idempotency_test(source)?;
            // Write js files for runtime test
            if source.source_type.is_javascript() {
                fs::write(&source.path, source_text).unwrap();
            }
            Ok(note)
        } else {
            Ok(None)
        }
    }

    fn driver(&self) -> Driver;

    /// Returns the idempotent output text together with an optional note to print.
    fn idempotency_test(
        &self,
        source: &Source,
    ) -> Result<(String, Option<String>), Vec<Diagnostic>> {
        let source_text = check_idempotency(self.name(), &|| self.driver(), source)?;
        Ok((source_text, None))
    }
}

/// Runs `driver` twice and returns the output if the second run doesn't change it.
pub fn check_idempotency(
    case: &'static str,
    driver: &dyn Fn() -> Driver,
    source: &Source,
) -> Result<String, Vec<Diagnostic>> {
    let Source { path, source_type, source_text } = source;
    let source_text2 = driver().run(path, source_text, *source_type)?;
    let source_text3 = driver().run(path, &source_text2, *source_type)?;
    if source_text2 != source_text3 {
        return Err(vec![Diagnostic {
            case,
            path: path.clone(),
            message: NodeModulesRunner::get_diff(&source_text2, &source_text3, false),
        }]);
    }
    Ok(source_text3)
}

/// Variable merging removes the declarations it made redundant, so mangling its output again
/// can pick different names. Checks the fixed point without merging and returns the merged
/// output for the runtime tests.
pub fn check_mangle_idempotency(
    case: &'static str,
    driver: &dyn Fn() -> Driver,
    source: &Source,
) -> Result<(String, Option<String>), Vec<Diagnostic>> {
    check_idempotency(case, &|| Driver { no_merge_vars: true, ..driver() }, source)?;
    let Source { path, source_type, source_text } = source;
    let source_text = driver().run(path, source_text, *source_type)?;
    Ok((source_text, None))
}
