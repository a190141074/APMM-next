//! Library persistence and schema migration boundary.

pub(crate) const CURRENT_SCHEMA: u32 = 1;

/// Ordered migration identifier. New schema changes must add a step instead
/// of relying on serde defaults alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MigrationStep {
    pub from: u32,
    pub to: u32,
}

pub(crate) const MIGRATIONS: &[MigrationStep] = &[];
