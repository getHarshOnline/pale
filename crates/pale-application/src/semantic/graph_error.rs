//! Typed failure boundary for source-free semantic projection.

/// Failure to project admitted Package meaning into one repository occurrence.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// Package-graph verification failed before repository projection began.
    #[error(transparent)]
    Package(#[from] crate::packages::PackageError),
    /// One or more independently actionable repository semantic contradictions.
    #[error(transparent)]
    Diagnostics(#[from] super::SemanticDiagnostics),
}

impl From<super::SemanticDiagnostic> for SemanticError {
    fn from(diagnostic: super::SemanticDiagnostic) -> Self {
        let mut accumulator = super::DiagnosticAccumulator::default();
        accumulator.push(diagnostic);
        Self::Diagnostics(
            accumulator
                .finish(())
                .expect_err("one pushed diagnostic cannot finish successfully"),
        )
    }
}
