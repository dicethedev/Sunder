pub mod handler;
pub mod assembler;

use assembler::SunderAssembler;
use sunder_core::audit::AuditLog;

pub struct AppState {
    pub assembler: SunderAssembler,
    pub audit: AuditLog,
}