pub mod handler;
pub mod node_signer;

use node_signer::NodeSigner;
use sunder_core::audit::AuditLog;

pub struct AppState {
    pub signer: NodeSigner,
    pub audit: AuditLog,
}