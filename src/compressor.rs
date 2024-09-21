use oxc::span::SourceType;

use crate::{Case, Driver};

pub struct CompressorRunner;

impl Case for CompressorRunner {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn run_test(&self, source_type: SourceType) -> bool {
        source_type.is_javascript() && !source_type.is_jsx()
    }

    fn driver(&self) -> Driver {
        Driver { compress: true, ..Driver::default() }
    }
}
