use oxc::span::SourceType;

use crate::{Case, Driver};

pub struct RemoveWhitespaceRunner;

impl Case for RemoveWhitespaceRunner {
    fn name(&self) -> &'static str {
        "RemoveWhitespace"
    }

    fn run_test(&self, source_type: SourceType) -> bool {
        source_type.is_javascript() && !source_type.is_jsx()
    }

    fn driver(&self) -> Driver {
        Driver { remove_whitespace: true, ..Driver::default() }
    }
}
