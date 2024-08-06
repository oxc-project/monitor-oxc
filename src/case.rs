use oxc::span::SourceType;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{Diagnostic, Driver, NodeModulesRunner, Source};

pub trait Case {
    fn name(&self) -> &'static str;

    fn save_file(&self, path: &Path, source_type: SourceType) -> Option<PathBuf>;

    fn test(&self, source: &Source) -> Result<(), Diagnostic> {
        let source_text = self.idempotency_test(source)?;
        // Write js files for runtime test
        if let Some(path) = self.save_file(&source.path, source.source_type) {
            fs::write(path, source_text).unwrap();
        }
        Ok(())
    }

    fn driver(&self) -> Driver;

    fn idempotency_test(&self, source: &Source) -> Result<String, Diagnostic> {
        let Source { path, source_type, source_text } = source;
        let source_text2 = self.driver().run(path, source_text, *source_type)?;
        let source_text3 = self.driver().run(path, &source_text2, *source_type)?;
        if source_text2 != source_text3 {
            return Err(Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: NodeModulesRunner::print_diff(&source_text2, &source_text3),
            });
        }
        Ok(source_text3)
    }
}
