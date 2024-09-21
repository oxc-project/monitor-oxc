use oxc::span::SourceType;

use crate::{Case, Driver};

pub struct ManglerRunner;

impl Case for ManglerRunner {
    fn name(&self) -> &'static str {
        "Mangler"
    }

    fn run_test(&self, source_type: SourceType) -> bool {
        source_type.is_javascript() && !source_type.is_jsx()
    }

    fn driver(&self) -> Driver {
        Driver { mangle: true, ..Driver::default() }
    }
}
