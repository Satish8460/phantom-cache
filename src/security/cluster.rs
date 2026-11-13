use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    pub node_id: u8,
    pub cluster_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterSecurityError {
    UntrustedCa,
    ExpiredCertificate,
    IdentityMismatch(String),
    ClusterMismatch { expected: String, found: String },
    RevokedNode(u8),
    UnknownNode(u8),
    FingerprintMismatch { expected: Vec<String>, found: String },
    Timeout,
    PlaintextRejected,
    MalformedIdentity(String),
    StaleRegistryEpoch { current: u64, incoming: u64 },
    CorruptClusterRegistry(String),
    NoClientCertificate,
    HandshakeFailed(String),
}

impl std::fmt::Display for ClusterSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UntrustedCa => write!(f, "Cluster mTLS connection rejected: untrusted CA authority"),
            Self::ExpiredCertificate => write!(f, "Cluster mTLS connection rejected: certificate expired"),
            Self::IdentityMismatch(msg) => write!(f, "Cluster mTLS identity mismatch: {}", msg),
            Self::ClusterMismatch { expected, found } => write!(
                f,
                "Cross-cluster connection rejected: expected cluster '{}', found '{}'",
                expected, found
            ),
            Self::RevokedNode(id) => write!(f, "Cluster mTLS node {} is revoked / inactive in registry", id),
            Self::UnknownNode(id) => write!(f, "Cluster mTLS node {} is unknown to membership registry", id),
            Self::FingerprintMismatch { expected, found } => write!(
                f,
                "Cluster mTLS certificate fingerprint mismatch: expected one of {:?}, found '{}'",
                expected, found
            ),
            Self::Timeout => write!(f, "Cluster mTLS handshake timed out (1000ms limit)"),
            Self::PlaintextRejected => write!(f, "Cluster mTLS listener rejected plaintext connection"),
            Self::MalformedIdentity(msg) => write!(f, "Malformed peer identity / SAN syntax: {}", msg),
            Self::StaleRegistryEpoch { current, incoming } => write!(
                f,
                "Registry update rejected: incoming epoch {} is not newer than current epoch {}",
                incoming, current
            ),
            Self::CorruptClusterRegistry(msg) => write!(f, "Corrupt cluster membership registry: {}", msg),
            Self::NoClientCertificate => write!(f, "Cluster mTLS peer did not present a client certificate"),
            Self::HandshakeFailed(msg) => write!(f, "Cluster mTLS handshake failed: {}", msg),
        }
    }
}

impl std::error::Error for ClusterSecurityError {}

/// Strict RFC 1035 / RFC 1123 SAN validator.
/// Format: `node-{node_id}.{cluster_id}.phantom.internal`
/// - `node_id`: Decimal integer `1..=255`.
/// - `cluster_id`: Alphanumeric lowercase string matching `^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$`.
///   Underscores `_` are strictly prohibited.
pub fn parse_san_node_identity(san: &str) -> Result<NodeIdentity, ClusterSecurityError> {
    let raw = san.strip_prefix("DNS:").or_else(|| san.strip_prefix("dns:")).unwrap_or(san).trim();

    // RFC 1035/1123 syntax rules: underscores are strictly forbidden in hostnames
    if raw.contains('_') {
        return Err(ClusterSecurityError::MalformedIdentity(format!(
            "SAN contains forbidden underscore character: '{}'",
            san
        )));
    }

    let suffix = ".phantom.internal";
    if !raw.ends_with(suffix) {
        return Err(ClusterSecurityError::MalformedIdentity(format!(
            "SAN domain must end with '{}', found '{}'",
            suffix, san
        )));
    }

    let prefix = &raw[..raw.len() - suffix.len()];
    let dot_pos = prefix.find('.').ok_or_else(|| {
        ClusterSecurityError::MalformedIdentity(format!(
            "SAN missing cluster_id label separator: '{}'",
            san
        ))
    })?;

    let node_part = &prefix[..dot_pos];
    let cluster_part = &prefix[dot_pos + 1..];

    if !node_part.starts_with("node-") {
        return Err(ClusterSecurityError::MalformedIdentity(format!(
            "SAN node label must start with 'node-', found '{}'",
            node_part
        )));
    }

    let id_str = &node_part["node-".len()..];
    let node_id: u8 = id_str.parse().map_err(|_| {
        ClusterSecurityError::MalformedIdentity(format!(
            "Invalid node_id in SAN '{}' (must be integer 1..=255)",
            id_str
        ))
    })?;

    if node_id == 0 {
        return Err(ClusterSecurityError::MalformedIdentity(
            "Node ID 0 is invalid (range is 1..=255)".to_string(),
        ));
    }

    // Validate cluster_id label syntax: ^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$
    if cluster_part.is_empty() || cluster_part.len() > 32 {
        return Err(ClusterSecurityError::MalformedIdentity(format!(
            "Cluster ID length {} is out of bounds (1..=32)",
            cluster_part.len()
        )));
    }

    if cluster_part.starts_with('-') || cluster_part.ends_with('-') {
        return Err(ClusterSecurityError::MalformedIdentity(format!(
            "Cluster ID cannot start or end with a hyphen: '{}'",
            cluster_part
        )));
    }

    for ch in cluster_part.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
            return Err(ClusterSecurityError::MalformedIdentity(format!(
                "Cluster ID contains invalid character '{}' (must be lowercase alphanumeric or hyphen)",
                ch
            )));
        }
    }

    Ok(NodeIdentity {
        node_id,
        cluster_id: cluster_part.to_string(),
    })
}

/// Helper function to read a single DER Tag-Length-Value (TLV) element safely.
fn read_der_tlv(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    if input.is_empty() {
        return None;
    }
    let tag = input[0];
    let mut idx = 1;
    if idx >= input.len() {
        return None;
    }
    let first_len = input[idx];
    idx += 1;
    let len = if first_len & 0x80 == 0 {
        first_len as usize
    } else {
        let num_bytes = (first_len & 0x7f) as usize;
        if num_bytes == 0 || num_bytes > 4 || idx + num_bytes > input.len() {
            return None;
        }
        let mut l = 0usize;
        for _ in 0..num_bytes {
            l = (l << 8) | (input[idx] as usize);
            idx += 1;
        }
        l
    };
    if idx + len > input.len() {
        return None;
    }
    let value = &input[idx..idx + len];
    let rest = &input[idx + len..];
    Some((tag, value, rest))
}

/// Extracts DNS Subject Alternative Names (SANs) from raw X.509 certificate DER bytes.
/// Standard RFC 5280 X.509:
/// - Certificate SEQUENCE
///   - TBSCertificate SEQUENCE
///     - Extensions [3] (0xa3)
///       - SEQUENCE OF Extension
///         - extnID OID 2.5.29.17 (`06 03 55 1d 11`)
///         - extnValue OCTET STRING (0x04)
///           - GeneralNames SEQUENCE (0x30)
///             - dNSName [2] IA5String (0x82)
pub fn extract_sans_from_der(cert_der: &[u8]) -> Result<Vec<String>, ClusterSecurityError> {
    // 1. Certificate SEQUENCE
    let (tag, cert_body, _) = read_der_tlv(cert_der).ok_or_else(|| {
        ClusterSecurityError::MalformedIdentity("Invalid DER: not a certificate SEQUENCE".to_string())
    })?;
    if tag != 0x30 {
        return Err(ClusterSecurityError::MalformedIdentity(
            "Certificate does not start with SEQUENCE (0x30)".to_string(),
        ));
    }

    // 2. TBSCertificate SEQUENCE
    let (tag, tbs_body, _) = read_der_tlv(cert_body).ok_or_else(|| {
        ClusterSecurityError::MalformedIdentity("Invalid DER: missing TBSCertificate".to_string())
    })?;
    if tag != 0x30 {
        return Err(ClusterSecurityError::MalformedIdentity(
            "TBSCertificate is not a SEQUENCE (0x30)".to_string(),
        ));
    }

    // 3. Find Extensions [3] constructed tag (0xa3) inside TBSCertificate
    let mut current_tbs = tbs_body;
    let mut extensions_raw: Option<&[u8]> = None;
    while let Some((elem_tag, elem_val, rest)) = read_der_tlv(current_tbs) {
        if elem_tag == 0xa3 {
            extensions_raw = Some(elem_val);
            break;
        }
        current_tbs = rest;
    }

    let extensions_raw = match extensions_raw {
        Some(e) => e,
        None => return Ok(Vec::new()), // No extensions present
    };

    // Extensions is a SEQUENCE of Extension
    let (tag, mut ext_list, _) = read_der_tlv(extensions_raw).ok_or_else(|| {
        ClusterSecurityError::MalformedIdentity("Invalid DER: extensions tag not a SEQUENCE".to_string())
    })?;
    if tag != 0x30 {
        return Err(ClusterSecurityError::MalformedIdentity(
            "Extensions wrapper is not a SEQUENCE".to_string(),
        ));
    }

    const SAN_OID: [u8; 3] = [0x55, 0x1d, 0x11]; // 2.5.29.17
    let mut sans = Vec::new();

    while let Some((ext_tag, ext_val, rest_ext)) = read_der_tlv(ext_list) {
        if ext_tag == 0x30 {
            // Extension SEQUENCE: (OID, optional BOOLEAN critical, OCTET STRING extnValue)
            let mut ext_parts = ext_val;
            let mut is_san = false;
            let mut extn_value: Option<&[u8]> = None;

            while let Some((p_tag, p_val, p_rest)) = read_der_tlv(ext_parts) {
                if p_tag == 0x06 && p_val == SAN_OID {
                    is_san = true;
                } else if p_tag == 0x04 {
                    extn_value = Some(p_val);
                }
                ext_parts = p_rest;
            }

            if is_san {
                if let Some(val_bytes) = extn_value {
                    // Inside OCTET STRING is GeneralNames (SEQUENCE 0x30)
                    if let Some((gn_tag, mut gn_body, _)) = read_der_tlv(val_bytes) {
                        if gn_tag == 0x30 {
                            while let Some((name_tag, name_val, rest_gn)) = read_der_tlv(gn_body) {
                                // 0x82 is context-specific tag [2] primitive (dNSName)
                                if name_tag == 0x82 {
                                    if let Ok(dns_name) = std::str::from_utf8(name_val) {
                                        sans.push(dns_name.to_string());
                                    }
                                }
                                gn_body = rest_gn;
                            }
                        }
                    }
                }
            }
        }
        ext_list = rest_ext;
    }

    Ok(sans)
}

/// Computes the lowercase hex-encoded SHA-256 fingerprint of the raw certificate DER bytes.
pub fn compute_cert_fingerprint(cert_der: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, cert_der);
    digest
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn default_revocation_channel() -> tokio::sync::broadcast::Sender<u8> {
    let (tx, _) = tokio::sync::broadcast::channel(64);
    tx
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemberNode {
    pub node_id: u8,
    pub fingerprints: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterMembershipRegistry {
    pub cluster_id: String,
    pub version: u64,
    pub members: HashMap<u8, MemberNode>,
    #[serde(default)]
    pub addresses: HashMap<u8, SocketAddr>,
    #[serde(skip, default = "default_revocation_channel")]
    pub revocation_tx: tokio::sync::broadcast::Sender<u8>,
}

impl PartialEq for ClusterMembershipRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.cluster_id == other.cluster_id
            && self.version == other.version
            && self.members == other.members
    }
}
impl Eq for ClusterMembershipRegistry {}

impl ClusterMembershipRegistry {
    pub fn new(cluster_id: impl Into<String>, version: u64) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            version,
            members: HashMap::new(),
            addresses: HashMap::new(),
            revocation_tx: default_revocation_channel(),
        }
    }

    pub fn get_address(&self, node_id: u8) -> Option<SocketAddr> {
        self.addresses.get(&node_id).copied()
    }

    pub fn set_address(&mut self, node_id: u8, addr: SocketAddr) {
        self.addresses.insert(node_id, addr);
    }

    pub fn subscribe_revocations(&self) -> tokio::sync::broadcast::Receiver<u8> {
        self.revocation_tx.subscribe()
    }

    pub fn add_member(&mut self, member: MemberNode) {
        let is_revoking = !member.active;
        let id = member.node_id;
        self.members.insert(member.node_id, member);
        if is_revoking {
            let _ = self.revocation_tx.send(id);
        }
    }

    pub fn get_member(&self, node_id: u8) -> Option<&MemberNode> {
        self.members.get(&node_id)
    }

    /// Evaluates Layer 3 (Cluster ID) and Layer 4 (Membership Admission) of the trust model.
    pub fn validate_peer(
        &self,
        identity: &NodeIdentity,
        leaf_cert_der: &[u8],
    ) -> Result<(), ClusterSecurityError> {
        // Layer 3: Cluster Identity Verification
        if identity.cluster_id != self.cluster_id {
            return Err(ClusterSecurityError::ClusterMismatch {
                expected: self.cluster_id.clone(),
                found: identity.cluster_id.clone(),
            });
        }

        // Layer 4: Membership Authorization (Admission Control)
        let member = self
            .members
            .get(&identity.node_id)
            .ok_or(ClusterSecurityError::UnknownNode(identity.node_id))?;

        if !member.active {
            return Err(ClusterSecurityError::RevokedNode(identity.node_id));
        }

        let cert_fingerprint = compute_cert_fingerprint(leaf_cert_der);
        let matches = member
            .fingerprints
            .iter()
            .any(|fp| fp.eq_ignore_ascii_case(&cert_fingerprint));

        if !matches {
            return Err(ClusterSecurityError::FingerprintMismatch {
                expected: member.fingerprints.clone(),
                found: cert_fingerprint,
            });
        }

        Ok(())
    }

    /// Updates the membership registry enforcing monotonic epoch increments.
    pub fn update(&mut self, incoming: ClusterMembershipRegistry) -> Result<(), ClusterSecurityError> {
        if incoming.cluster_id != self.cluster_id {
            return Err(ClusterSecurityError::ClusterMismatch {
                expected: self.cluster_id.clone(),
                found: incoming.cluster_id,
            });
        }

        if incoming.version <= self.version {
            return Err(ClusterSecurityError::StaleRegistryEpoch {
                current: self.version,
                incoming: incoming.version,
            });
        }

        let mut revoked = Vec::new();
        for (&id, member) in self.members.iter() {
            if member.active {
                match incoming.members.get(&id) {
                    Some(new_member) if !new_member.active => revoked.push(id),
                    None => revoked.push(id),
                    _ => {}
                }
            }
        }

        let tx = self.revocation_tx.clone();
        *self = incoming;
        self.revocation_tx = tx;

        for id in revoked {
            let _ = self.revocation_tx.send(id);
        }

        Ok(())
    }
}

/// JSON configuration schema for cluster membership file (e.g., `cluster-members.json`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterMembersConfigFile {
    pub cluster_id: String,
    pub version: u64,
    pub members: Vec<MemberNode>,
    #[serde(default)]
    pub addresses: HashMap<u8, String>,
}

fn validate_fingerprint_hex(fp: &str) -> Result<(), ClusterSecurityError> {
    if fp.len() != 64 {
        return Err(ClusterSecurityError::CorruptClusterRegistry(format!(
            "Fingerprint '{}' has invalid length {} (expected 64 hex characters for SHA-256)",
            fp,
            fp.len()
        )));
    }
    for ch in fp.chars() {
        if !ch.is_ascii_digit() && !matches!(ch, 'a'..='f') {
            return Err(ClusterSecurityError::CorruptClusterRegistry(format!(
                "Fingerprint '{}' contains invalid character '{}' (must be lowercase hex 0-9, a-f)",
                fp, ch
            )));
        }
    }
    Ok(())
}

/// Validates and parses a cluster membership registry from a JSON string.
/// Enforces:
/// - Cluster ID matching expected cluster.
/// - Valid RFC 1035 label syntax for cluster ID.
/// - Node IDs in range 1..=255.
/// - Rejection of duplicate node IDs.
/// - Strict 64-character lowercase SHA-256 hex format for all fingerprints.
/// - Non-empty fingerprints list per member.
pub fn load_membership_from_json_str(
    json_str: &str,
    expected_cluster_id: &str,
) -> Result<ClusterMembershipRegistry, ClusterSecurityError> {
    // Try deserializing as array-of-members schema, or map-of-members schema
    let (cluster_id, version, members_vec, addrs_map) = if let Ok(cfg) = serde_json::from_str::<ClusterMembersConfigFile>(json_str) {
        (cfg.cluster_id, cfg.version, cfg.members, cfg.addresses)
    } else if let Ok(reg) = serde_json::from_str::<ClusterMembershipRegistry>(json_str) {
        let members: Vec<MemberNode> = reg.members.into_values().collect();
        let addrs: HashMap<u8, String> = reg.addresses.into_iter().map(|(k, v)| (k, v.to_string())).collect();
        (reg.cluster_id, reg.version, members, addrs)
    } else {
        return Err(ClusterSecurityError::CorruptClusterRegistry(
            "Malformed JSON: unable to parse cluster membership configuration".to_string(),
        ));
    };

    if cluster_id != expected_cluster_id {
        return Err(ClusterSecurityError::ClusterMismatch {
            expected: expected_cluster_id.to_string(),
            found: cluster_id,
        });
    }

    if version == 0 {
        return Err(ClusterSecurityError::CorruptClusterRegistry(
            "Registry version must be a positive integer >= 1".to_string(),
        ));
    }

    let mut registry = ClusterMembershipRegistry::new(cluster_id, version);
    let mut seen_ids = std::collections::HashSet::new();

    for member in members_vec {
        if member.node_id == 0 {
            return Err(ClusterSecurityError::CorruptClusterRegistry(
                "Node ID 0 is invalid (allowed range is 1..=255)".to_string(),
            ));
        }

        if !seen_ids.insert(member.node_id) {
            return Err(ClusterSecurityError::CorruptClusterRegistry(format!(
                "Duplicate node_id {} found in membership configuration",
                member.node_id
            )));
        }

        if member.fingerprints.is_empty() {
            return Err(ClusterSecurityError::CorruptClusterRegistry(format!(
                "Node {} has empty fingerprints list",
                member.node_id
            )));
        }

        for fp in &member.fingerprints {
            validate_fingerprint_hex(fp)?;
        }

        registry.add_member(member);
    }

    for (id, addr_str) in addrs_map {
        let sa = addr_str.parse::<SocketAddr>().map_err(|e| {
            ClusterSecurityError::CorruptClusterRegistry(format!(
                "Invalid socket address '{}' for node {}: {}",
                addr_str, id, e
            ))
        })?;
        registry.set_address(id, sa);
    }

    Ok(registry)
}

/// Loads and validates cluster membership configuration from a file path.
/// Note: File-based membership configuration is authoritative at node startup;
/// updates to the file require process restart to reload.
pub fn load_membership_from_file<P: AsRef<std::path::Path>>(
    path: P,
    expected_cluster_id: &str,
) -> Result<ClusterMembershipRegistry, ClusterSecurityError> {
    let p = path.as_ref();
    let content = std::fs::read_to_string(p).map_err(|e| {
        ClusterSecurityError::CorruptClusterRegistry(format!(
            "Failed to read cluster membership file {}: {}",
            p.display(),
            e
        ))
    })?;
    load_membership_from_json_str(&content, expected_cluster_id)
}

/// Mutual TLS configuration for PhantomCache cluster nodes.
#[derive(Clone)]
pub struct ClusterTlsConfig {
    pub node_id: u8,
    pub cluster_id: String,
    pub server_config: Arc<rustls::ServerConfig>,
    pub client_config: Arc<rustls::ClientConfig>,
    pub registry: Arc<RwLock<ClusterMembershipRegistry>>,
}

impl ClusterTlsConfig {
    pub fn from_pem(
        node_id: u8,
        cluster_id: &str,
        ca_pem: &[u8],
        cert_pem: &[u8],
        key_pem: &[u8],
        registry: ClusterMembershipRegistry,
    ) -> Result<Self, ClusterSecurityError> {
        // 1. Root Cert Store containing ONLY the cluster CA (WebPKI strictly excluded)
        let mut root_store = rustls::RootCertStore::empty();
        let mut ca_reader = std::io::BufReader::new(ca_pem);
        let ca_certs = rustls_pemfile::certs(&mut ca_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to parse CA PEM: {}", e)))?;

        if ca_certs.is_empty() {
            return Err(ClusterSecurityError::UntrustedCa);
        }

        for cert in ca_certs {
            root_store.add(cert).map_err(|e| {
                ClusterSecurityError::HandshakeFailed(format!("Failed to add CA cert: {}", e))
            })?;
        }

        // 2. Parse Node Certificate and Private Key
        let mut cert_reader = std::io::BufReader::new(cert_pem);
        let node_certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to parse node cert PEM: {}", e)))?;

        if node_certs.is_empty() {
            return Err(ClusterSecurityError::MalformedIdentity("No node certs found".to_string()));
        }

        let mut key_reader = std::io::BufReader::new(key_pem);
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to parse private key: {}", e)))?
            .ok_or_else(|| ClusterSecurityError::HandshakeFailed("No private key found in PEM".to_string()))?;

        // 3. Server Config: Mandatory Client Authentication via WebPkiClientVerifier (Strict TLS 1.3 Only)
        let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store.clone()))
            .build()
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to build client verifier: {}", e)))?;

        let server_config = rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(node_certs.clone(), private_key.clone_key())
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to build server config: {}", e)))?;

        // 4. Client Config: Outbound mTLS with Cluster CA and Node Client Cert (Strict TLS 1.3 Only)
        let client_config = rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_root_certificates(root_store)
            .with_client_auth_cert(node_certs, private_key)
            .map_err(|e| ClusterSecurityError::HandshakeFailed(format!("Failed to build client config: {}", e)))?;

        Ok(Self {
            node_id,
            cluster_id: cluster_id.to_string(),
            server_config: Arc::new(server_config),
            client_config: Arc::new(client_config),
            registry: Arc::new(RwLock::new(registry)),
        })
    }

    pub fn server_acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::from(self.server_config.clone())
    }

    pub fn client_connector(&self) -> TlsConnector {
        TlsConnector::from(self.client_config.clone())
    }

    pub fn subscribe_revocations(&self) -> tokio::sync::broadcast::Receiver<u8> {
        self.registry.read().unwrap().subscribe_revocations()
    }
}

/// Enforces fail-closed cluster transport security invariants at server startup.
/// On network interfaces, cluster communication strictly requires mutual TLS anchored to PHANTOM_CLUSTER_CA.
/// Network deployments cannot bypass mTLS. Loopback bindings permit local development when unconfigured.
pub fn validate_cluster_startup_security(
    bind_addr: &SocketAddr,
    has_cluster_ca: bool,
    has_node_cert: bool,
    has_node_key: bool,
    allow_insecure_cluster: bool,
) -> Result<(), crate::security::StartupSecurityError> {
    if !bind_addr.ip().is_loopback() {
        // Network-accessible interface: cluster mTLS is non-negotiable and strictly mandatory!
        if !has_cluster_ca || !has_node_cert || !has_node_key {
            return Err(crate::security::StartupSecurityError::NetworkWithoutClusterTls(*bind_addr));
        }
    } else {
        // On loopback: if allow_insecure_cluster is explicitly false and no TLS is configured, fail closed
        let any_cert = has_cluster_ca || has_node_cert || has_node_key;
        let all_cert = has_cluster_ca && has_node_cert && has_node_key;
        if any_cert && !all_cert {
            return Err(crate::security::StartupSecurityError::NetworkWithoutClusterTls(*bind_addr));
        }
        if !all_cert && !allow_insecure_cluster && std::env::var("PHANTOM_STRICT_CLUSTER").is_ok() {
            return Err(crate::security::StartupSecurityError::LoopbackClusterWithoutInsecureOverride);
        }
    }
    Ok(())
}
