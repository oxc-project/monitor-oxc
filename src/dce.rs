use oxc::minifier::CompressOptions;

use crate::{Case, Driver, Source};

pub struct DceRunner;

impl Case for DceRunner {
    fn name(&self) -> &'static str {
        "DCE"
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver { compress: Some(CompressOptions::dead_code_elimination()), ..Driver::default() }
    }
}
