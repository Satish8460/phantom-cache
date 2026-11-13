pub mod cluster;
pub mod jwt;
pub mod tls;
pub mod upstream;

use std::net::SocketAddr;
use std::sync::Arc;

pub use cluster::{
    load_membership_from_file, load_membership_from_json_str, parse_san_node_identity,
    validate_cluster_startup_security, ClusterMembershipRegistry, ClusterSecurityError,
    ClusterTlsConfig, MemberNode, NodeIdentity,
};
pub use jwt::{AuthError, JwtClaims, JwtValidator, Role};
pub use tls::TlsServerConfig;
pub use upstream::{
    resolve_and_pin_origin, validate_port, OriginSecurityError, PreConnectIpFilter, UpstreamPolicy,
    UpstreamTlsConfig, HARD_BLOCKED_PORTS, STANDARD_ALLOWED_PORTS,
};

#[derive(Clone)]
pub struct SecurityConfig {
    pub jwt_validator: Option<jwt::JwtValidator>,
    pub require_auth_for_control: bool,
    pub enforce_rfc9111_auth_bypass: bool,
    pub upstream: Option<Arc<UpstreamPolicy>>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_validator: None,
            require_auth_for_control: true,
            enforce_rfc9111_auth_bypass: true,
            upstream: None,
        }
    }
}

impl SecurityConfig {
    pub fn new_with_jwt(validator: jwt::JwtValidator) -> Self {
        Self {
            jwt_validator: Some(validator),
            require_auth_for_control: true,
            enforce_rfc9111_auth_bypass: true,
            upstream: None,
        }
    }

    pub fn with_upstream(mut self, upstream: Arc<UpstreamPolicy>) -> Self {
        self.upstream = Some(upstream);
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartupSecurityError {
    NetworkWithoutAuth(SocketAddr),
    NetworkWithoutTls(SocketAddr),
    RequireAuthWithoutSecret,
    NetworkInsecureUpstreamWithoutAllowlist(SocketAddr),
    NetworkWithoutClusterTls(SocketAddr),
    LoopbackClusterWithoutInsecureOverride,
}

impl std::fmt::Display for StartupSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkWithoutAuth(addr) => write!(
                f,
                "Fail-Closed: Server configured to bind to network-accessible address {}, but PHANTOM_JWT_SECRET is not configured. Network deployments must fail closed.",
                addr
            ),
            Self::NetworkWithoutTls(addr) => write!(
                f,
                "Fail-Closed: Server configured to bind to network-accessible address {} without TLS certificates. Set PHANTOM_TLS_CERT/KEY or PHANTOM_ALLOW_INSECURE_HTTP=true to override.",
                addr
            ),
            Self::RequireAuthWithoutSecret => write!(
                f,
                "Fail-Closed: PHANTOM_REQUIRE_AUTH is enabled, but PHANTOM_JWT_SECRET is missing."
            ),
            Self::NetworkInsecureUpstreamWithoutAllowlist(addr) => write!(
                f,
                "Fail-Closed: Server configured to bind to network-accessible address {} with PHANTOM_ALLOW_INSECURE_UPSTREAM enabled, but PHANTOM_ORIGIN_ALLOWLIST is empty. Network deployments must fail closed.",
                addr
            ),
            Self::NetworkWithoutClusterTls(addr) => write!(
                f,
                "Fail-Closed: Server configured to bind to network-accessible address {} without cluster mTLS certificates (PHANTOM_CLUSTER_CA, PHANTOM_NODE_CERT, PHANTOM_NODE_KEY). Network cluster deployments must fail closed.",
                addr
            ),
            Self::LoopbackClusterWithoutInsecureOverride => write!(
                f,
                "Fail-Closed: Loopback cluster node started without mTLS certificates and without PHANTOM_ALLOW_INSECURE_CLUSTER=true."
            ),
        }
    }
}

impl std::error::Error for StartupSecurityError {}

/// Enforces fail-closed security invariants at server startup.
/// - Loopback bindings (`127.0.0.1` / `::1`) permit local unauthenticated development unless `require_auth` is set.
/// - Network-accessible bindings (`!addr.ip().is_loopback()`) strictly fail closed if authentication or TLS is unconfigured.
/// - Lab Override (`allow_insecure_http`): Permits plain HTTP on a network interface ONLY if valid authentication (`has_security`) is present.
///   It is restricted to demo labs and NEVER bypasses mandatory JWT authentication.
pub fn validate_startup_security(
    addr: &SocketAddr,
    has_security: bool,
    has_tls: bool,
    require_auth_flag: bool,
    allow_insecure_http: bool,
) -> Result<(), StartupSecurityError> {
    if require_auth_flag && !has_security {
        return Err(StartupSecurityError::RequireAuthWithoutSecret);
    }

    if !addr.ip().is_loopback() {
        // MANDATORY: Network deployments MUST have authentication configured.
        // allow_insecure_http can NEVER bypass this check.
        if !has_security {
            return Err(StartupSecurityError::NetworkWithoutAuth(*addr));
        }
        if !has_tls && !allow_insecure_http {
            return Err(StartupSecurityError::NetworkWithoutTls(*addr));
        }
    }

    Ok(())
}

/// Enforces fail-closed upstream security invariants at server startup.
/// On network interfaces, enabling insecure plaintext HTTP upstream strictly requires an explicit allowlist.
pub fn validate_upstream_startup_security(
    addr: &SocketAddr,
    allow_insecure_upstream: bool,
    has_origin_allowlist: bool,
) -> Result<(), StartupSecurityError> {
    if !addr.ip().is_loopback() && allow_insecure_upstream && !has_origin_allowlist {
        return Err(StartupSecurityError::NetworkInsecureUpstreamWithoutAllowlist(*addr));
    }
    Ok(())
}

