use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "cache:read")]
    CacheRead,
    #[serde(rename = "cache:invalidate")]
    CacheInvalidate,
    #[serde(rename = "telemetry:read")]
    Observer,
    #[serde(rename = "scenario:operate")]
    Operator,
    #[serde(rename = "admin")]
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::CacheRead => "cache:read",
            Role::CacheInvalidate => "cache:invalidate",
            Role::Observer => "telemetry:read",
            Role::Operator => "scenario:operate",
            Role::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    #[serde(default)]
    pub iat: Option<u64>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl JwtClaims {
    pub fn new(sub: &str, iss: &str, aud: &str, exp: u64, tenant_id: Option<&str>, roles: Vec<&str>) -> Self {
        Self {
            sub: sub.to_string(),
            iss: iss.to_string(),
            aud: aud.to_string(),
            exp,
            iat: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            tenant_id: tenant_id.map(|s| s.to_string()),
            roles: roles.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn has_role(&self, target_role: Role) -> bool {
        // Admin role satisfies all permissions
        if self.roles.iter().any(|r| r.eq_ignore_ascii_case("admin")) {
            return true;
        }

        match target_role {
            Role::CacheRead => self.roles.iter().any(|r| {
                r.eq_ignore_ascii_case("cache:read")
                    || r.eq_ignore_ascii_case("requester")
                    || r.eq_ignore_ascii_case("read")
            }),
            Role::CacheInvalidate => self.roles.iter().any(|r| {
                r.eq_ignore_ascii_case("cache:invalidate")
                    || r.eq_ignore_ascii_case("requester")
                    || r.eq_ignore_ascii_case("invalidate")
            }),
            Role::Observer => self.roles.iter().any(|r| {
                r.eq_ignore_ascii_case("telemetry:read")
                    || r.eq_ignore_ascii_case("observer")
            }),
            Role::Operator => self.roles.iter().any(|r| {
                r.eq_ignore_ascii_case("scenario:operate")
                    || r.eq_ignore_ascii_case("operator")
            }),
            Role::Admin => self.roles.iter().any(|r| r.eq_ignore_ascii_case("admin")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    ExpiredToken,
    InvalidIssuer,
    InvalidAudience,
    InsufficientPermissions,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing Bearer authorization token"),
            AuthError::InvalidToken(msg) => write!(f, "Invalid authorization token: {}", msg),
            AuthError::ExpiredToken => write!(f, "Authorization token has expired"),
            AuthError::InvalidIssuer => write!(f, "Token issuer (iss) mismatch"),
            AuthError::InvalidAudience => write!(f, "Token audience (aud) mismatch"),
            AuthError::InsufficientPermissions => write!(f, "Insufficient permissions for requested resource"),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Clone)]
pub struct JwtValidator {
    pub issuer: String,
    pub audience: String,
    pub secret: Vec<u8>,
    pub rotation_secrets: HashMap<String, Vec<u8>>,
}

impl JwtValidator {
    pub fn new(issuer: &str, audience: &str, secret: &[u8]) -> Self {
        Self {
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            secret: secret.to_vec(),
            rotation_secrets: HashMap::new(),
        }
    }

    pub fn with_rotation_key(mut self, kid: &str, secret: &[u8]) -> Self {
        self.rotation_secrets.insert(kid.to_string(), secret.to_vec());
        self
    }

    pub fn create_token(&self, claims: &JwtClaims) -> Result<String, jsonwebtoken::errors::Error> {
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
        let key = jsonwebtoken::EncodingKey::from_secret(&self.secret);
        jsonwebtoken::encode(&header, claims, &key)
    }

    pub fn create_token_with_kid(&self, kid: &str, claims: &JwtClaims) -> Result<String, jsonwebtoken::errors::Error> {
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
        header.kid = Some(kid.to_string());
        let secret = self.rotation_secrets.get(kid).unwrap_or(&self.secret);
        let key = jsonwebtoken::EncodingKey::from_secret(secret);
        jsonwebtoken::encode(&header, claims, &key)
    }

    pub fn validate(&self, token_str: &str) -> Result<JwtClaims, AuthError> {
        let header = jsonwebtoken::decode_header(token_str)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        if header.alg != jsonwebtoken::Algorithm::HS256 {
            return Err(AuthError::InvalidToken("Unsupported signing algorithm".to_string()));
        }

        let secret = if let Some(ref kid) = header.kid {
            match self.rotation_secrets.get(kid) {
                Some(s) => s,
                None => {
                    return Err(AuthError::InvalidToken(format!("Unknown key ID (kid): {}", kid)));
                }
            }
        } else {
            &self.secret
        };

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.leeway = 10; // 10 second clock skew tolerance

        let decoding_key = jsonwebtoken::DecodingKey::from_secret(secret);
        let token_data = jsonwebtoken::decode::<JwtClaims>(token_str, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => AuthError::InvalidIssuer,
                jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::InvalidAudience,
                _ => AuthError::InvalidToken(e.to_string()),
            })?;

        let claims = token_data.claims;

        // Strict verification of issuer and audience in case validation parameters were relaxed
        if claims.iss != self.issuer {
            return Err(AuthError::InvalidIssuer);
        }
        if claims.aud != self.audience {
            return Err(AuthError::InvalidAudience);
        }

        Ok(claims)
    }
}

