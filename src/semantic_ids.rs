use std::ops::ControlFlow;

use oxc::{
    CompilerInterface,
    ast::ast::BindingIdentifier,
    ast_visit::{Visit, walk},
    diagnostics::OxcDiagnostic,
    semantic::SemanticBuilderReturn,
};

use crate::{Case, Diagnostic, Driver, Source};

pub struct SemanticSymbolIdsRunner;

impl Case for SemanticSymbolIdsRunner {
    fn name(&self) -> &'static str {
        "SemanticSymbolIds"
    }

    fn enable_runtime_test(&self) -> bool {
        false
    }

    fn test(&self, source: &Source) -> Result<(), Vec<Diagnostic>> {
        if !self.run_test(source) {
            return Ok(());
        }

        let mut driver = SemanticIdsCheckDriver::new(source);
        driver.compile(&source.source_text, source.source_type, &source.path);

        if driver.checker.errors.is_empty() { Ok(()) } else { Err(driver.checker.errors) }
    }

    fn driver(&self) -> Driver {
        Driver::default()
    }
}

struct SemanticIdsCheckDriver<'a> {
    checker: SemanticIdsChecker<'a>,
}

impl<'s> SemanticIdsCheckDriver<'s> {
    fn new(source: &'s Source) -> Self {
        Self { checker: SemanticIdsChecker { source, errors: vec![] } }
    }
}

impl CompilerInterface for SemanticIdsCheckDriver<'_> {
    fn handle_errors(&mut self, errors: Vec<OxcDiagnostic>) {
        // Ignore parse/semantic errors - we only care about missing symbol IDs
        // and reference IDs on successfully parsed identifiers
        let _ = errors;
    }

    fn after_semantic(&mut self, semantic_return: &mut SemanticBuilderReturn) -> ControlFlow<()> {
        self.checker.visit_program(semantic_return.semantic.nodes().program());
        ControlFlow::Continue(())
    }
}

struct SemanticIdsChecker<'a> {
    source: &'a Source,
    errors: Vec<Diagnostic>,
}

impl<'a> Visit<'a> for SemanticIdsChecker<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if it.symbol_id.get().is_none() {
            self.errors.push(Diagnostic {
                case: "SemanticSymbolIds",
                path: self.source.path.clone(),
                message: format!(
                    "BindingIdentifier '{}' at {:?} has no symbol_id",
                    it.name, it.span
                ),
            });
        }
        walk::walk_binding_identifier(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &oxc::ast::ast::IdentifierReference<'a>) {
        if it.reference_id.get().is_none() {
            self.errors.push(Diagnostic {
                case: "SemanticSymbolIds",
                path: self.source.path.clone(),
                message: format!(
                    "IdentifierReference '{}' at {:?} has no reference_id",
                    it.name, it.span
                ),
            });
        }
        walk::walk_identifier_reference(self, it);
    }
}
