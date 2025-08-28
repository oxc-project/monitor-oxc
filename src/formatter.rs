use oxc::{allocator::Allocator, parser::Parser};
use oxc_formatter::{FormatOptions, Formatter};

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
        let program = Parser::new(&allocator, source_text, *source_type).parse().program;
        let source_text2 = Formatter::new(&allocator, FormatOptions::default()).build(&program);
        let program = Parser::new(&allocator, &source_text2, *source_type).parse().program;
        let source_text3 = Formatter::new(&allocator, FormatOptions::default()).build(&program);
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
