use crate::{Case, Driver, Source};

pub struct CompressorRunner;

impl Case for CompressorRunner {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn run_test(&self, source: &Source) -> bool {
        source.is_js_only()
    }

    fn driver(&self) -> Driver {
        Driver { compress: true, ..Driver::default() }
    }
}
