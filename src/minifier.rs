use oxc::minifier::CompressOptions;

use crate::{Case, Driver, Source};

pub struct MinifierRunner;

impl Case for MinifierRunner {
    fn name(&self) -> &'static str {
        "Minifier"
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver {
            compress: Some(CompressOptions::default()),
            mangle: true,
            remove_whitespace: true,
            ..Driver::default()
        }
    }
}
