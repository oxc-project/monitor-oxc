use crate::{Case, Driver, Source};

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
}
