use std::path::{Path, PathBuf};

use oxc::span::SourceType;

use crate::{Case, Driver};

pub struct CompressorRunner;

impl Case for CompressorRunner {
    fn name(&self) -> &'static str {
        "Compressor"
    }

    fn save_file(&self, path: &Path, source_type: SourceType) -> Option<PathBuf> {
        source_type.is_javascript().then(|| path.to_path_buf())
    }

    fn driver(&self) -> Driver {
        Driver { compress: true, ..Driver::default() }
    }
}
