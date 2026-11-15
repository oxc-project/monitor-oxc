use crate::{Case, Diagnostic, Driver, Source, case::check_mangle_idempotency};

pub struct ManglerRunner;

impl Case for ManglerRunner {
    fn name(&self) -> &'static str {
        "Mangler"
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver { mangle: true, ..Driver::default() }
    }

    fn idempotency_test(
        &self,
        source: &Source,
    ) -> Result<(String, Option<String>), Vec<Diagnostic>> {
        check_mangle_idempotency(self.name(), &|| self.driver(), source)
    }
}
