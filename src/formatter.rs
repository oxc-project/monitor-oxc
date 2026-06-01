use oxc::allocator::Allocator;
use oxc_formatter::{format_program, parse_for_format, JsFormatOptions};

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
        let parsed1 = parse_for_format(&allocator, source_text, *source_type);
        let errors1 = parsed1.errors;
        if !errors1.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error in original source: {errors1:?}"),
            }]);
        }

        let source_text2 =
            format_program(&allocator, &parsed1.program, JsFormatOptions::default(), None)
                .print()
                .map_err(|err| {
                    vec![Diagnostic {
                        case: self.name(),
                        path: path.clone(),
                        message: format!("Format error in original source: {err:?}"),
                    }]
                })?
                .into_code();

        let parsed2 = parse_for_format(&allocator, &source_text2, *source_type);
        let errors2 = parsed2.errors;
        if !errors2.is_empty() {
            return Err(vec![Diagnostic {
                case: self.name(),
                path: path.clone(),
                message: format!("Parse error after formatting: {errors2:?}"),
            }]);
        }

        let source_text3 =
            format_program(&allocator, &parsed2.program, JsFormatOptions::default(), None)
                .print()
                .map_err(|err| {
                    vec![Diagnostic {
                        case: self.name(),
                        path: path.clone(),
                        message: format!("Format error after formatting: {err:?}"),
                    }]
                })?
                .into_code();

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
