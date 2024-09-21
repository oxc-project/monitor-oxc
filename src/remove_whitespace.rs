use crate::{Case, Driver, Source};

pub struct RemoveWhitespaceRunner;

impl Case for RemoveWhitespaceRunner {
    fn name(&self) -> &'static str {
        "RemoveWhitespace"
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver { remove_whitespace: true, ..Driver::default() }
    }
}
