use oxc::allocator::Allocator;
use oxc_formatter::JsFormatOptions;

use crate::{Case, Diagnostic, Driver, NodeModulesRunner, Source};

pub struct FormatterRunner;

impl Case for FormatterRunner {
    fn name(&self) -> &'static str {
        "Formatter"
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn driver(&self) -> Driver {
        unreachable!()
    }

    fn idempotency_test(&self, source: &Source) -> Result<String, Vec<Diagnostic>> {
        let Source { path, source_type, source_text } = source;

        let allocator = Allocator::new();

        let source_text2 = match oxc_formatter::format(
            &allocator,
            source_text,
            *source_type,
            JsFormatOptions::default(),
            None,
        ) {
            Ok(formatted) => formatted.print().unwrap().into_code(),
            Err(error) => {
                return Err(vec![Diagnostic {
                    case: self.name(),
                    path: path.clone(),
                    message: format!("Parse error in original source: {error:?}"),
                }]);
            }
        };

        let allocator2 = Allocator::new();
        let source_text3 = match oxc_formatter::format(
            &allocator2,
            &source_text2,
            *source_type,
            JsFormatOptions::default(),
            None,
        ) {
            Ok(formatted) => formatted.print().unwrap().into_code(),
            Err(error) => {
                return Err(vec![Diagnostic {
                    case: self.name(),
                    path: path.clone(),
                    message: format!("Parse error after formatting: {error:?}"),
                }]);
            }
        };

        if source_text2 != source_text3 {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: NodeModulesRunner::get_diff(&source_text2, &source_text3, false),
            }]);
        }

        Ok(source_text3)
    }
}
