//! IPC command boundary.
//!
//! Commands are migrated here incrementally. Their names and wire structures
//! are intentionally kept stable during the v0.3.0 refactor.

pub(crate) const IPC_SCHEMA: u32 = 1;
