use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

pub const GOVERNING_LICENSE: &str = "GNU Affero General Public License version 3 or later (AGPLv3+)";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseIdentity {
    pub binary_version: String,
    pub source_revision: Option<String>,
    pub spec_context: Option<String>,
    pub tree_state: String,
    pub build_time: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseMetadata {
    pub latest_release_identity: String,
    pub release_gate_status: String,
    pub accepted_verification_status: String,
    pub artifact_availability: Vec<String>,
    pub verification_environment: String,
    pub credibility_location: String,
}

pub fn current_release_identity() -> ReleaseIdentity {
    ReleaseIdentity {
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
        source_revision: BUILD_REVISION.map(str::to_string),
        spec_context: BUILD_SPEC_CONTEXT.map(str::to_string),
        tree_state: BUILD_TREE_STATE.to_string(),
        build_time: BUILD_TIME.map(str::to_string),
    }
}

pub fn format_release_identity_human(identity: &ReleaseIdentity) -> String {
    let mut parts = vec![format!("version {}", identity.binary_version)];
    if let Some(revision) = identity.source_revision.as_deref() {
        parts.push(format!("revision {}", short_revision(revision)));
    }
    if let Some(spec_context) = identity.spec_context.as_deref() {
        parts.push(format!("spec {}", spec_context));
    }
    if identity.tree_state != "clean" {
        parts.push(identity.tree_state.clone());
    }
    parts.join(" | ")
}

pub fn long_version_text() -> &'static str {
    static LONG_VERSION: OnceLock<String> = OnceLock::new();
    LONG_VERSION.get_or_init(|| {
        let identity = current_release_identity();
        let mut version = identity.binary_version;
        if let Some(revision) = identity.source_revision.as_deref() {
            version.push_str(&format!(" ({})", short_revision(revision)));
        }
        if let Some(spec_context) = identity.spec_context.as_deref() {
            version.push_str(&format!(" [spec: {spec_context}]"));
        }
        if identity.tree_state != "clean" {
            version.push_str(&format!(" {}", identity.tree_state));
        }
        version
    })
}

pub fn cli_license_notice() -> &'static str {
    GOVERNING_LICENSE
}

pub fn short_revision(revision: &str) -> &str {
    &revision[..revision.len().min(8)]
}

#[cfg(test)]
mod tests {
    use super::{cli_license_notice, current_release_identity, long_version_text, GOVERNING_LICENSE};

    #[test]
    fn current_release_identity_uses_package_version() {
        let identity = current_release_identity();
        assert_eq!(identity.binary_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn long_version_includes_package_version() {
        assert!(long_version_text().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn cli_license_notice_mentions_agpl() {
        assert!(cli_license_notice().contains("AGPLv3+"));
        assert_eq!(cli_license_notice(), GOVERNING_LICENSE);
    }
}
