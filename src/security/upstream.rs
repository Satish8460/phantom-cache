use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio_rustls::TlsConnector;

pub const HARD_BLOCKED_PORTS: &[u16] = &[
    20, 21,    // FTP
    22,        // SSH
    23,        // Telnet
    25,        // SMTP
    53,        // DNS
    69,        // TFTP
    110, 143,  // POP3, IMAP
    137, 138, 139, 445, // NetBIOS, SMB
    514,       // Syslog
    2049,      // NFS
    2375, 2376,// Docker daemon
    3306,      // MySQL
    5432,      // PostgreSQL
    6379,      // Redis
    11211,     // Memcached
    27017,     // MongoDB
    9200, 9300,// Elasticsearch
    8500,      // Consul
    2379, 2380,// etcd
];

pub const STANDARD_ALLOWED_PORTS: &[u16] = &[80, 443, 8080, 8443, 9000, 9443];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginSecurityError {
    InvalidOriginUrl(String),
    DisallowedScheme(String),
    DisallowedPort(u16),
    SensitivePortBlocked(u16),
    OriginNotAllowlisted(String),
    InsecureUpstreamDisallowed,
    SsrfBlocked(String),
    TlsHandshakeFailed(String),
    TlsCertificateExpired,
    TlsHostnameMismatch(String),
    TlsUntrustedRoot,
    Timeout,
    MalformedFraming(String),
    TruncatedResponse(String),
    OversizedResponse(String),
    NetworkError(String),
}

impl std::fmt::Display for OriginSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOriginUrl(msg) => write!(f, "Invalid origin URL: {}", msg),
            Self::DisallowedScheme(s) => write!(f, "Scheme '{}' is disallowed by security policy", s),
            Self::DisallowedPort(p) => write!(f, "Port {} is disallowed by security policy", p),
            Self::SensitivePortBlocked(p) => write!(f, "Sensitive port {} is unconditionally blocked", p),
            Self::OriginNotAllowlisted(o) => write!(f, "Origin '{}' is not in configured allowlist", o),
            Self::InsecureUpstreamDisallowed => write!(f, "Insecure HTTP upstream is disallowed in network mode"),
            Self::SsrfBlocked(msg) => write!(f, "SSRF destination blocked: {}", msg),
            Self::TlsHandshakeFailed(msg) => write!(f, "Upstream TLS handshake failed: {}", msg),
            Self::TlsCertificateExpired => write!(f, "Upstream TLS certificate has expired"),
            Self::TlsHostnameMismatch(msg) => write!(f, "Upstream TLS hostname mismatch: {}", msg),
            Self::TlsUntrustedRoot => write!(f, "Upstream TLS certificate is signed by an untrusted authority"),
            Self::Timeout => write!(f, "Upstream origin timed out"),
            Self::MalformedFraming(msg) => write!(f, "Malformed upstream HTTP response framing: {}", msg),
            Self::TruncatedResponse(msg) => write!(f, "Upstream HTTP response was truncated: {}", msg),
            Self::OversizedResponse(msg) => write!(f, "Upstream response exceeds size limit: {}", msg),
            Self::NetworkError(msg) => write!(f, "Upstream network error: {}", msg),
        }
    }
}

impl std::error::Error for OriginSecurityError {}

pub fn normalize_origin(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let lower = lower.trim_end_matches('/');
    if let Ok(parsed) = url::Url::parse(lower) {
        let scheme = parsed.scheme();
        let host = parsed.host_str().unwrap_or("").trim_end_matches('.');
        let port = parsed.port_or_known_default().unwrap_or(if scheme == "http" { 80 } else { 443 });
        format!("{}://{}:{}", scheme, host, port)
    } else {
        lower.trim_end_matches('.').to_string()
    }
}

pub fn validate_port(port: u16, origin_url: &str, allowlist: &[String]) -> Result<(), OriginSecurityError> {
    // 1. HARD BLOCKED SENSITIVE PORTS: Non-negotiable, non-bypassable!
    if HARD_BLOCKED_PORTS.contains(&port) {
        return Err(OriginSecurityError::SensitivePortBlocked(port));
    }

    // 2. Standard web and alternative web ports are permitted
    if STANDARD_ALLOWED_PORTS.contains(&port) {
        return Ok(());
    }

    // 3. Privileged system ports (< 1024) outside standard list are always blocked
    if port < 1024 {
        return Err(OriginSecurityError::DisallowedPort(port));
    }

    // 4. Custom unprivileged ports (>= 1024):
    // If no allowlist is configured (default local dev/testing), non-sensitive ports are permitted.
    // If an allowlist is configured, custom unprivileged ports must match an allowlisted origin.
    if allowlist.is_empty() {
        return Ok(());
    }

    let norm = normalize_origin(origin_url);
    if allowlist.iter().any(|entry| normalize_origin(entry) == norm) {
        Ok(())
    } else {
        Err(OriginSecurityError::DisallowedPort(port))
    }
}

#[derive(Clone)]
pub struct PreConnectIpFilter {
    pub allow_loopback: bool,
    pub allow_lan: bool,
}

impl PreConnectIpFilter {
    pub fn new(allow_loopback: bool, allow_lan: bool) -> Self {
        Self { allow_loopback, allow_lan }
    }

    pub fn check_ip(&self, ip: IpAddr) -> Result<(), OriginSecurityError> {
        match ip {
            IpAddr::V4(v4) => self.check_ipv4(v4),
            IpAddr::V6(v6) => self.check_ipv6(v6),
        }
    }

    pub fn check_ipv4(&self, ip: Ipv4Addr) -> Result<(), OriginSecurityError> {
        let octets = ip.octets();

        // 1. Cloud Metadata IMDS (169.254.169.254/32) - ALWAYS BLOCKED
        if octets == [169, 254, 169, 254] {
            return Err(OriginSecurityError::SsrfBlocked(
                "Cloud metadata IMDS (169.254.169.254) is strictly prohibited".to_string(),
            ));
        }

        // 2. Link-Local (169.254.0.0/16) - ALWAYS BLOCKED
        if octets[0] == 169 && octets[1] == 254 {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv4 link-local (169.254.0.0/16) is blocked".to_string(),
            ));
        }

        // 3. Loopback (127.0.0.0/8)
        if octets[0] == 127 {
            if !self.allow_loopback {
                return Err(OriginSecurityError::SsrfBlocked(
                    "Loopback address (127.0.0.0/8) is blocked".to_string(),
                ));
            }
            return Ok(());
        }

        // 4. Unspecified / Broadcast (0.0.0.0/8, 255.255.255.255)
        if ip.is_unspecified() || ip.is_broadcast() || octets[0] == 0 {
            return Err(OriginSecurityError::SsrfBlocked(
                "Unspecified or broadcast IPv4 is blocked".to_string(),
            ));
        }

        // 5. Multicast (224.0.0.0/4)
        if ip.is_multicast() {
            return Err(OriginSecurityError::SsrfBlocked(
                "Multicast IPv4 is blocked".to_string(),
            ));
        }

        // 6. Carrier-Grade NAT (100.64.0.0/10)
        if octets[0] == 100 && (octets[1] >= 64 && octets[1] <= 127) {
            return Err(OriginSecurityError::SsrfBlocked(
                "Carrier-Grade NAT (100.64.0.0/10) is blocked".to_string(),
            ));
        }

        // 7. Documentation / Test Nets (RFC 5737)
        if (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
            || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
            || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        {
            return Err(OriginSecurityError::SsrfBlocked(
                "Documentation test network IPv4 is blocked".to_string(),
            ));
        }

        // 8. Private RFC 1918 (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
        let is_private = octets[0] == 10
            || (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31))
            || (octets[0] == 192 && octets[1] == 168);

        if is_private && !self.allow_lan {
            return Err(OriginSecurityError::SsrfBlocked(
                "RFC 1918 private network address is blocked by default".to_string(),
            ));
        }

        Ok(())
    }

    pub fn check_ipv6(&self, ip: Ipv6Addr) -> Result<(), OriginSecurityError> {
        // IPv6 Loopback (::1) - MUST check before to_ipv4() because to_ipv4() treats ::1 as IPv4 0.0.0.1
        if ip.is_loopback() {
            if !self.allow_loopback {
                return Err(OriginSecurityError::SsrfBlocked(
                    "IPv6 loopback address (::1) is blocked".to_string(),
                ));
            }
            return Ok(());
        }

        // IPv6 Unspecified (::)
        if ip.is_unspecified() {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 unspecified address (::) is blocked".to_string(),
            ));
        }

        // 1. Unmap both IPv4-compatible (::x.x.x.x) and IPv4-mapped (::ffff:x.x.x.x) addresses
        if let Some(v4) = ip.to_ipv4() {
            return self.check_ipv4(v4);
        }

        // IPv6 Multicast (ff00::/8)
        if ip.is_multicast() {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 multicast address is blocked".to_string(),
            ));
        }

        let segments = ip.segments();

        // 2. IPv6 6to4 transition prefix (2002::/16, RFC 3056)
        // Embedded IPv4 is contained in bits 16..48 (segments 1 and 2)
        if segments[0] == 0x2002 {
            let embedded_v4 = Ipv4Addr::new(
                (segments[1] >> 8) as u8,
                segments[1] as u8,
                (segments[2] >> 8) as u8,
                segments[2] as u8,
            );
            return self.check_ipv4(embedded_v4);
        }

        // 3. IPv6 NAT64 Well-Known Prefix (64:ff9b::/96, RFC 6052)
        // Embedded IPv4 is contained in the lowest 32 bits (segments 6 and 7)
        if segments[0] == 0x0064
            && segments[1] == 0xff9b
            && segments[2] == 0
            && segments[3] == 0
            && segments[4] == 0
            && segments[5] == 0
        {
            let embedded_v4 = Ipv4Addr::new(
                (segments[6] >> 8) as u8,
                segments[6] as u8,
                (segments[7] >> 8) as u8,
                segments[7] as u8,
            );
            return self.check_ipv4(embedded_v4);
        }

        // 4. IPv6 Link-Local (fe80::/10)
        if (segments[0] & 0xffc0) == 0xfe80 {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 link-local address (fe80::/10) is blocked".to_string(),
            ));
        }

        // 5. AWS IMDSv2 IPv6: fd00:ec2::254
        if segments[0] == 0xfd00 && segments[1] == 0x0ec2 && segments[7] == 0x0254 {
            return Err(OriginSecurityError::SsrfBlocked(
                "Cloud metadata IMDS IPv6 (fd00:ec2::254) is strictly prohibited".to_string(),
            ));
        }

        // 6. IPv6 ULA (fc00::/7)
        if (segments[0] & 0xfe00) == 0xfc00 && !self.allow_lan {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 Unique Local Address (ULA fc00::/7) is blocked by default".to_string(),
            ));
        }

        // 7. IPv6 Documentation (2001:db8::/32)
        if segments[0] == 0x2001 && segments[1] == 0x0db8 {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 documentation test network (2001:db8::/32) is blocked".to_string(),
            ));
        }

        // 8. IPv6 Discard-Only prefix (100::/64, RFC 6666)
        if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
            return Err(OriginSecurityError::SsrfBlocked(
                "IPv6 discard-only prefix (100::/64) is blocked".to_string(),
            ));
        }

        Ok(())
    }
}

pub async fn resolve_and_pin_origin(
    host: &str,
    port: u16,
    filter: &PreConnectIpFilter,
) -> Result<SocketAddr, OriginSecurityError> {
    let target = format!("{}:{}", host, port);
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&target)
        .await
        .map_err(|e| OriginSecurityError::NetworkError(format!("DNS resolution failed for {}: {}", host, e)))?
        .collect();

    if addrs.is_empty() {
        return Err(OriginSecurityError::NetworkError(format!(
            "DNS lookup for {} returned no IP addresses",
            host
        )));
    }

    // CRITICAL: Check EVERY address in the returned DNS answer set!
    // If a DNS response contains even one prohibited address (e.g. mixed public + metadata IP),
    // the entire answer set is tainted and must be rejected to prevent rebinding / bypass attacks.
    for addr in &addrs {
        filter.check_ip(addr.ip())?;
    }

    // For localhost, prefer IPv4 loopback to ensure compatibility with standard IPv4 listeners
    if addrs.len() > 1 && host.eq_ignore_ascii_case("localhost") {
        if let Some(v4) = addrs.iter().find(|a| a.is_ipv4()) {
            return Ok(*v4);
        }
    }

    // Pin the first validated address for the socket connection
    Ok(addrs[0])
}

pub struct UpstreamTlsConfig {
    pub connector: TlsConnector,
}

impl UpstreamTlsConfig {
    pub fn new_with_root_store(root_store: rustls::RootCertStore) -> Self {
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        Self {
            connector: TlsConnector::from(Arc::new(config)),
        }
    }

    pub fn new_default() -> Self {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Self::new_with_root_store(root_store)
    }

    pub fn new_with_ca_pem(ca_pem: &[u8]) -> Result<Self, OriginSecurityError> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let mut reader = std::io::BufReader::new(ca_pem);
        let certs = rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OriginSecurityError::TlsHandshakeFailed(format!("Failed to parse CA PEM: {}", e)))?;

        for cert in certs {
            root_store.add(cert).map_err(|e| {
                OriginSecurityError::TlsHandshakeFailed(format!("Failed to add CA cert: {}", e))
            })?;
        }

        Ok(Self::new_with_root_store(root_store))
    }
}

#[derive(Clone)]
pub struct UpstreamPolicy {
    pub origin_url: Option<String>,
    pub allowed_origins: Vec<String>,
    pub enforce_tls: bool,
    pub allow_insecure_upstream: bool,
    pub allow_lan_origin: bool,
    pub allow_loopback_origin: bool,
    pub forward_client_authorization: bool,
    pub tls_config: Option<Arc<UpstreamTlsConfig>>,
}

impl Default for UpstreamPolicy {
    fn default() -> Self {
        Self {
            origin_url: None,
            allowed_origins: Vec::new(),
            enforce_tls: false,
            allow_insecure_upstream: false,
            allow_lan_origin: false,
            allow_loopback_origin: true,
            forward_client_authorization: false,
            tls_config: None,
        }
    }
}

impl UpstreamPolicy {
    pub fn validate_target(&self, parsed: &url::Url) -> Result<(), OriginSecurityError> {
        let scheme = parsed.scheme();
        if scheme != "https" && scheme != "http" {
            return Err(OriginSecurityError::DisallowedScheme(scheme.to_string()));
        }

        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(OriginSecurityError::InvalidOriginUrl(
                "Userinfo in origin URL is forbidden".to_string(),
            ));
        }

        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(OriginSecurityError::InvalidOriginUrl(
                "Query or fragment in base origin URL is forbidden".to_string(),
            ));
        }

        let port = parsed.port_or_known_default().unwrap_or(443);
        let mut check_allowlist = self.allowed_origins.clone();
        if let Some(ref o) = self.origin_url {
            check_allowlist.push(o.clone());
        }
        validate_port(port, parsed.as_str(), &check_allowlist)?;

        // Scheme enforcement
        if scheme == "http" {
            if self.enforce_tls {
                return Err(OriginSecurityError::DisallowedScheme(
                    "HTTPS is strictly enforced by policy".to_string(),
                ));
            }

            let host_str = parsed.host_str().unwrap_or("");
            // Use the parsed host: `host_str()` keeps the brackets of IPv6 literals ("[::1]"),
            // so a string comparison with "::1" never matched (M7 defect fix).
            let is_loopback = match parsed.host() {
                Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
                Some(url::Host::Ipv4(a)) => a.is_loopback(),
                Some(url::Host::Ipv6(a)) => a.is_loopback(),
                None => false,
            };

            if is_loopback {
                if !self.allow_loopback_origin {
                    return Err(OriginSecurityError::InsecureUpstreamDisallowed);
                }
            } else {
                // Network-accessible insecure HTTP strictly requires allow_insecure_upstream AND allowlist entry
                if !self.allow_insecure_upstream {
                    return Err(OriginSecurityError::InsecureUpstreamDisallowed);
                }
                let origin_normalized = format!("http://{}:{}", host_str, port);
                let is_allowlisted = self.allowed_origins.iter().any(|allowed| {
                    normalize_origin(allowed) == origin_normalized
                });
                if !is_allowlisted {
                    return Err(OriginSecurityError::OriginNotAllowlisted(origin_normalized));
                }
            }
        }

        // Global allowlist check if allowlist is populated
        if !self.allowed_origins.is_empty() {
            let host_str = parsed.host_str().unwrap_or("");
            let norm = format!("{}://{}:{}", scheme, host_str, port);
            if !self.allowed_origins.iter().any(|allowed| normalize_origin(allowed) == norm) {
                return Err(OriginSecurityError::OriginNotAllowlisted(norm));
            }
        }

        Ok(())
    }
}

