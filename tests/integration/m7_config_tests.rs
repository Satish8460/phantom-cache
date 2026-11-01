//! M7-01 configuration validation matrix, precedence, redaction, idempotence and
//! docs-vs-code checks. Pure: no process environment is read or modified.

use phantom_cache_demo::config::*;
use std::collections::{BTreeMap, BTreeSet};

const S32: &str = "0123456789abcdef0123456789abcdef"; // test-only 32-byte value
const CANARY: &str = "CANARY-SECRET-9f3a7c-do-not-leak-0000";

fn env(pairs: &[(&str, &str)]) -> Inputs {
    Inputs {
        env: pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        ..Default::default()
    }
}

fn ok(pairs: &[(&str, &str)]) -> Config {
    resolve(&env(pairs)).unwrap_or_else(|e| panic!("expected valid {:?}: {:?}", pairs, e))
}

fn err(pairs: &[(&str, &str)]) -> Vec<ConfigError> {
    match resolve(&env(pairs)) {
        Ok(_) => panic!("expected invalid {:?}", pairs),
        Err(e) => e,
    }
}

#[test]
fn defaults_resolve_and_match_schema() {
    let c = ok(&[]);
    for k in KEYS {
        assert_eq!(c.get(k.name), k.default, "default of {}", k.name);
        assert_eq!(c.source(k.name), Source::Default);
    }
    assert_eq!(c.bind_addr().to_string(), "127.0.0.1:8085");
    // CLAUDE.md configuration reference defaults.
    assert_eq!(c.get_u64("PHANTOM_MAX_STALE_MS"), 86_400_000);
    assert_eq!(c.get_u64("PHANTOM_DEFAULT_TTL_MS"), 60_000);
    assert_eq!(c.get_u64("PHANTOM_SWEEPER_BATCH_SIZE"), 32);
    assert_eq!(c.get_u64("PHANTOM_SWEEPER_INTERVAL_MS"), 100);
    assert_eq!(c.get_u64("PHANTOM_MAX_CACHE_BYTES"), 268_435_456);
    assert!(c.get("PHANTOM_ORIGIN_CAP_MAX").is_none(), "origin cap must be unconfigured by default");
}

type Env = Vec<(&'static str, &'static str)>;

/// (key, valid env, invalid env). Every schema key must appear.
fn matrix() -> Vec<(&'static str, Env, Env)> {
    let full_cluster = vec![
        ("PHANTOM_CLUSTER_CA", "ca.pem"),
        ("PHANTOM_NODE_CERT", "n.pem"),
        ("PHANTOM_NODE_KEY", "k.pem"),
        ("PHANTOM_CLUSTER_MEMBERS_FILE", "m.json"),
        ("PHANTOM_CLUSTER_LISTEN_ADDR", "127.0.0.1:9101"),
    ];
    let with = |extra: (&'static str, &'static str)| {
        let mut v: Vec<_> = full_cluster.iter().filter(|(k, _)| *k != extra.0).cloned().collect();
        v.push(extra);
        v
    };
    vec![
        ("PHANTOM_MODE", vec![("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)], vec![("PHANTOM_MODE", "staging")]),
        ("PHANTOM_BIND_ADDR", vec![("PHANTOM_BIND_ADDR", "127.0.0.1:9000")], vec![("PHANTOM_BIND_ADDR", "localhost:8085")]),
        ("PHANTOM_TLS_CERT", vec![("PHANTOM_TLS_CERT", "a"), ("PHANTOM_TLS_KEY", "b")], vec![("PHANTOM_TLS_CERT", "a")]),
        ("PHANTOM_TLS_KEY", vec![("PHANTOM_TLS_CERT", "a"), ("PHANTOM_TLS_KEY", "b")], vec![("PHANTOM_TLS_KEY", "b")]),
        ("PHANTOM_ALLOW_INSECURE_HTTP", vec![("PHANTOM_ALLOW_INSECURE_HTTP", "true")], vec![("PHANTOM_ALLOW_INSECURE_HTTP", "maybe")]),
        ("PHANTOM_REQUIRE_AUTH", vec![("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)], vec![("PHANTOM_REQUIRE_AUTH", "true")]),
        ("PHANTOM_JWT_SECRET", vec![("PHANTOM_JWT_SECRET", S32)], vec![("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", "short")]),
        ("PHANTOM_JWT_ISSUER", vec![("PHANTOM_JWT_ISSUER", "iss")], vec![("PHANTOM_JWT_ISSUER", "  ")]),
        ("PHANTOM_JWT_AUDIENCE", vec![("PHANTOM_JWT_AUDIENCE", "aud")], vec![("PHANTOM_JWT_AUDIENCE", "  ")]),
        ("PHANTOM_MAX_CACHE_BYTES", vec![("PHANTOM_MAX_CACHE_BYTES", "1048576")], vec![("PHANTOM_MAX_CACHE_BYTES", "0")]),
        ("PHANTOM_DEFAULT_TTL_MS", vec![("PHANTOM_DEFAULT_TTL_MS", "5000")], vec![("PHANTOM_DEFAULT_TTL_MS", "abc")]),
        ("PHANTOM_MAX_STALE_MS", vec![("PHANTOM_MAX_STALE_MS", "1000")], vec![("PHANTOM_MAX_STALE_MS", "0")]),
        ("PHANTOM_SWEEPER_BATCH_SIZE", vec![("PHANTOM_SWEEPER_BATCH_SIZE", "16")], vec![("PHANTOM_SWEEPER_BATCH_SIZE", "2000")]),
        ("PHANTOM_SWEEPER_INTERVAL_MS", vec![("PHANTOM_SWEEPER_INTERVAL_MS", "50")], vec![("PHANTOM_SWEEPER_INTERVAL_MS", "0")]),
        ("ORIGIN_URL", vec![("ORIGIN_URL", "http://127.0.0.1:9000")], vec![("ORIGIN_URL", "ftp://x.example.com")]),
        ("PHANTOM_ORIGIN_ALLOWLIST", vec![("PHANTOM_ORIGIN_ALLOWLIST", "https://a.example.com")], vec![("PHANTOM_ORIGIN_ALLOWLIST", "not a url")]),
        ("PHANTOM_ENFORCE_UPSTREAM_TLS", vec![("PHANTOM_ENFORCE_UPSTREAM_TLS", "true")], vec![("PHANTOM_ENFORCE_UPSTREAM_TLS", "2")]),
        ("PHANTOM_ALLOW_INSECURE_UPSTREAM", vec![("PHANTOM_ALLOW_INSECURE_UPSTREAM", "true")], vec![("PHANTOM_ALLOW_INSECURE_UPSTREAM", "yes")]),
        ("PHANTOM_ALLOW_LAN_ORIGIN", vec![("PHANTOM_ALLOW_LAN_ORIGIN", "true")], vec![("PHANTOM_ALLOW_LAN_ORIGIN", "on")]),
        ("PHANTOM_FORWARD_UPSTREAM_AUTH", vec![("PHANTOM_FORWARD_UPSTREAM_AUTH", "true")], vec![("PHANTOM_FORWARD_UPSTREAM_AUTH", "nope")]),
        ("PHANTOM_UPSTREAM_CA_CERT", vec![("PHANTOM_UPSTREAM_CA_CERT", "ca.pem")], vec![("PHANTOM_UPSTREAM_CA_CERT", "  ")]),
        ("PHANTOM_CLUSTER_ID", vec![("PHANTOM_CLUSTER_ID", "prod-1")], vec![("PHANTOM_CLUSTER_ID", "Bad_ID")]),
        ("PHANTOM_CLUSTER_MEMBERS_FILE", vec![("PHANTOM_CLUSTER_MEMBERS_FILE", "m.json")], vec![("PHANTOM_CLUSTER_MEMBERS_FILE", "  ")]),
        ("PHANTOM_CLUSTER_CA", full_cluster.clone(), with(("PHANTOM_CLUSTER_CA", "ca.pem")).into_iter().filter(|(k, _)| *k != "PHANTOM_NODE_KEY").collect()),
        ("PHANTOM_NODE_CERT", full_cluster.clone(), full_cluster.iter().filter(|(k, _)| *k != "PHANTOM_NODE_CERT").cloned().collect()),
        ("PHANTOM_NODE_KEY", full_cluster.clone(), full_cluster.iter().filter(|(k, _)| *k != "PHANTOM_NODE_KEY").cloned().collect()),
        ("PHANTOM_NODE_ID", vec![("PHANTOM_NODE_ID", "7")], vec![("PHANTOM_NODE_ID", "0")]),
        ("PHANTOM_CLUSTER_LISTEN_ADDR", full_cluster.clone(), with(("PHANTOM_CLUSTER_LISTEN_ADDR", "nonsense"))),
        ("PHANTOM_CLUSTER_PEER_ADDRS", vec![("PHANTOM_CLUSTER_PEER_ADDRS", "2=127.0.0.1:9102")], vec![("PHANTOM_CLUSTER_PEER_ADDRS", "2=notaddr")]),
        ("PHANTOM_ALLOW_INSECURE_CLUSTER", vec![("PHANTOM_ALLOW_INSECURE_CLUSTER", "true")], vec![("PHANTOM_ALLOW_INSECURE_CLUSTER", "x")]),
        ("PHANTOM_STRICT_CLUSTER", vec![("PHANTOM_STRICT_CLUSTER", "true"), ("PHANTOM_ALLOW_INSECURE_CLUSTER", "true")], vec![("PHANTOM_STRICT_CLUSTER", "true"), ("PHANTOM_CLUSTER_MEMBERS_FILE", "m.json")]),
        ("PHANTOM_CLUSTER_TIMEOUT_MS", vec![("PHANTOM_CLUSTER_TIMEOUT_MS", "200")], vec![("PHANTOM_CLUSTER_TIMEOUT_MS", "0")]),
        ("PHANTOM_INTELLIGENCE", vec![("PHANTOM_INTELLIGENCE", "observe")], vec![("PHANTOM_INTELLIGENCE", "turbo")]),
        ("PHANTOM_SIMULATION", vec![("PHANTOM_SIMULATION", "true")], vec![("PHANTOM_SIMULATION", "maybe")]),
        ("PHANTOM_ORIGIN_CAP_MAX", vec![("PHANTOM_ORIGIN_CAP_MAX", "64")], vec![("PHANTOM_ORIGIN_CAP_MAX", "0")]),
        ("PHANTOM_ORIGIN_CAP_MIN", vec![("PHANTOM_ORIGIN_CAP_MAX", "64"), ("PHANTOM_ORIGIN_CAP_MIN", "4")], vec![("PHANTOM_ORIGIN_CAP_MAX", "8"), ("PHANTOM_ORIGIN_CAP_MIN", "9")]),
        ("PHANTOM_ORIGIN_CAP_P99_CEILING_MS", vec![("PHANTOM_ORIGIN_CAP_P99_CEILING_MS", "500")], vec![("PHANTOM_ORIGIN_CAP_P99_CEILING_MS", "0")]),
        ("PHANTOM_ORIGIN_CAP_WINDOW", vec![("PHANTOM_ORIGIN_CAP_WINDOW", "50")], vec![("PHANTOM_ORIGIN_CAP_WINDOW", "0")]),
        ("PHANTOM_CORS_ORIGINS", vec![("PHANTOM_CORS_ORIGINS", "https://ui.example.com,http://localhost:3000")], vec![("PHANTOM_CORS_ORIGINS", "*")]),
        ("PHANTOM_LAB_ENABLED", vec![("PHANTOM_LAB_ENABLED", "true")], vec![("PHANTOM_LAB_ENABLED", "maybe")]),
        ("PHANTOM_LAB_DIR", vec![("PHANTOM_LAB_DIR", "lab-out")], vec![("PHANTOM_LAB_DIR", "  ")]),
        ("PHANTOM_LAB_GATEWAY_EXE", vec![("PHANTOM_LAB_GATEWAY_EXE", "gw.exe")], vec![("PHANTOM_LAB_GATEWAY_EXE", "  ")]),
        ("PHANTOM_CONFIG", vec![("PHANTOM_CONFIG", "x.conf")], vec![("PHANTOM_CONFIG", "  ")]),
    ]
}

#[test]
fn validation_matrix_covers_every_key_with_valid_and_invalid_case() {
    let m = matrix();
    let covered: BTreeSet<&str> = m.iter().map(|(k, _, _)| *k).collect();
    for k in KEYS {
        assert!(covered.contains(k.name), "no matrix case for {}", k.name);
    }
    for (key, good, bad) in &m {
        resolve(&env(good)).unwrap_or_else(|e| panic!("{}: valid case rejected: {:?}", key, e));
        assert!(resolve(&env(bad)).is_err(), "{}: invalid case accepted: {:?}", key, bad);
    }
}

#[test]
fn single_network_node_does_not_require_cluster_material() {
    // Regression for the release-blocking defect: the former unconditional
    // validate_cluster_startup_security call rejected every non-loopback bind.
    let c = ok(&[
        ("PHANTOM_BIND_ADDR", "0.0.0.0:8443"),
        ("PHANTOM_JWT_SECRET", S32),
        ("PHANTOM_TLS_CERT", "c.pem"),
        ("PHANTOM_TLS_KEY", "k.pem"),
    ]);
    assert!(!c.cluster_enabled());
    // The security function itself still demands mTLS on a network address, so without
    // the cluster_enabled() gate it would have failed.
    let addr: std::net::SocketAddr = "0.0.0.0:8443".parse().unwrap();
    assert!(phantom_cache_demo::security::validate_cluster_startup_security(&addr, false, false, false, false).is_err());
}

#[test]
fn network_cluster_without_mtls_material_fails() {
    let e = err(&[
        ("PHANTOM_BIND_ADDR", "0.0.0.0:8443"),
        ("PHANTOM_JWT_SECRET", S32),
        ("PHANTOM_TLS_CERT", "c.pem"),
        ("PHANTOM_TLS_KEY", "k.pem"),
        ("PHANTOM_CLUSTER_MEMBERS_FILE", "m.json"),
    ]);
    assert!(e.iter().any(|x| x.message.contains("cluster mTLS")), "{:?}", e);
}

#[test]
fn conflict_rules() {
    // network bind without auth / without TLS
    assert!(!err(&[("PHANTOM_BIND_ADDR", "0.0.0.0:8085")]).is_empty());
    assert!(err(&[("PHANTOM_BIND_ADDR", "0.0.0.0:8085"), ("PHANTOM_JWT_SECRET", S32)])
        .iter()
        .any(|e| e.message.contains("without TLS")));
    ok(&[("PHANTOM_BIND_ADDR", "0.0.0.0:8085"), ("PHANTOM_JWT_SECRET", S32), ("PHANTOM_ALLOW_INSECURE_HTTP", "true")]);
    // insecure http never bypasses auth
    assert!(!err(&[("PHANTOM_BIND_ADDR", "0.0.0.0:8085"), ("PHANTOM_ALLOW_INSECURE_HTTP", "true")]).is_empty());
    // insecure upstream on network needs allowlist
    assert!(!err(&[
        ("PHANTOM_BIND_ADDR", "0.0.0.0:8085"),
        ("PHANTOM_JWT_SECRET", S32),
        ("PHANTOM_ALLOW_INSECURE_HTTP", "true"),
        ("PHANTOM_ALLOW_INSECURE_UPSTREAM", "true")
    ])
    .is_empty());
    // origin rejected by the same policy used at request time
    assert!(err(&[("ORIGIN_URL", "http://example.com:80")]).iter().any(|e| e.key.as_deref() == Some("ORIGIN_URL")));
    assert!(err(&[("ORIGIN_URL", "https://example.com"), ("PHANTOM_ORIGIN_ALLOWLIST", "https://other.example.com")])
        .iter()
        .any(|e| e.key.as_deref() == Some("ORIGIN_URL")));
    // cluster mTLS complete but members file missing
    assert!(err(&[
        ("PHANTOM_CLUSTER_CA", "a"),
        ("PHANTOM_NODE_CERT", "b"),
        ("PHANTOM_NODE_KEY", "c"),
        ("PHANTOM_CLUSTER_LISTEN_ADDR", "127.0.0.1:9101"),
    ])
    .iter()
    .any(|e| e.message.contains("MEMBERS_FILE")));
}

#[test]
fn production_profile() {
    let secure = [("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)];
    let c = ok(&secure);
    assert!(c.is_production());
    for extra in [
        ("PHANTOM_SIMULATION", "true"),
        ("PHANTOM_ALLOW_INSECURE_UPSTREAM", "true"),
        ("PHANTOM_ALLOW_INSECURE_CLUSTER", "true"),
        ("PHANTOM_FORWARD_UPSTREAM_AUTH", "true"),
    ] {
        let mut v = secure.to_vec();
        v.push(extra);
        assert!(err(&v).iter().any(|e| e.key.as_deref() == Some("PHANTOM_MODE")), "{:?}", extra);
    }
    assert!(!err(&[("PHANTOM_MODE", "production")]).is_empty());
}

#[test]
fn jwt_secret_length_rules() {
    // short secret on loopback without require_auth: warning only (back-compat)
    let c = ok(&[("PHANTOM_JWT_SECRET", "short")]);
    assert!(c.warnings.iter().any(|w| w.contains("PHANTOM_JWT_SECRET")));
    // strict when auth is required or the bind is a network address
    assert!(!err(&[("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", "short")]).is_empty());
    assert!(!err(&[("PHANTOM_BIND_ADDR", "0.0.0.0:1"), ("PHANTOM_JWT_SECRET", "short"), ("PHANTOM_ALLOW_INSECURE_HTTP", "true")]).is_empty());
}

#[test]
fn booleans_reject_typos_instead_of_reading_false() {
    // Previously PHANTOM_REQUIRE_AUTH=yes was silently read as false (auth off).
    for v in ["yes", "on", "enabled", "tru", "2"] {
        assert!(!err(&[("PHANTOM_REQUIRE_AUTH", v)]).is_empty(), "{}", v);
    }
    for v in ["TRUE", "True", "1"] {
        assert!(ok(&[("PHANTOM_SIMULATION", v)]).get_bool("PHANTOM_SIMULATION"));
    }
    for v in ["false", "FALSE", "0"] {
        assert!(!ok(&[("PHANTOM_SIMULATION", v)]).get_bool("PHANTOM_SIMULATION"));
    }
}

#[test]
fn origin_cap_semantics() {
    let c = ok(&[("PHANTOM_ORIGIN_CAP_MAX", "32"), ("PHANTOM_INTELLIGENCE", "canary"), ("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE", "true")]);
    assert_eq!(c.get_u64("PHANTOM_ORIGIN_CAP_MIN"), 1);
    assert_eq!(c.get_u64("PHANTOM_ORIGIN_CAP_P99_CEILING_MS"), 1000);
    assert_eq!(c.get_u64("PHANTOM_ORIGIN_CAP_WINDOW"), 100);
    assert!(!c.warnings.iter().any(|w| w.contains("inert")));
    let inert = ok(&[("PHANTOM_ORIGIN_CAP_MAX", "32")]);
    assert!(inert.warnings.iter().any(|w| w.contains("inert")), "cap without canary/active must warn");
    let orphan = ok(&[("PHANTOM_ORIGIN_CAP_WINDOW", "10")]);
    assert!(orphan.warnings.iter().any(|w| w.contains("not configured")));
}

#[test]
fn precedence_cli_env_file_default() {
    let inputs = Inputs {
        env: [("PHANTOM_JWT_ISSUER", "from-env"), ("PHANTOM_JWT_AUDIENCE", "env-aud")]
            .iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        file: vec![
            ("PHANTOM_JWT_ISSUER".into(), "from-file".into()),
            ("PHANTOM_JWT_AUDIENCE".into(), "file-aud".into()),
            ("PHANTOM_MAX_STALE_MS".into(), "5000".into()),
            ("PHANTOM_BIND_ADDR".into(), "127.0.0.1:7000".into()),
        ],
        cli: vec![("PHANTOM_BIND_ADDR", "127.0.0.1:7001".into())],
    };
    let c = resolve(&inputs).unwrap();
    assert_eq!((c.get("PHANTOM_BIND_ADDR"), c.source("PHANTOM_BIND_ADDR")), (Some("127.0.0.1:7001"), Source::Cli));
    assert_eq!((c.get("PHANTOM_JWT_ISSUER"), c.source("PHANTOM_JWT_ISSUER")), (Some("from-env"), Source::Env));
    assert_eq!((c.get("PHANTOM_MAX_STALE_MS"), c.source("PHANTOM_MAX_STALE_MS")), (Some("5000"), Source::File));
    assert_eq!(c.source("PHANTOM_SWEEPER_BATCH_SIZE"), Source::Default);
    // Empty env value counts as unset at that layer and falls through to the file.
    let mut i2 = inputs.clone();
    i2.env.insert("PHANTOM_JWT_ISSUER".into(), String::new());
    assert_eq!(resolve(&i2).unwrap().get("PHANTOM_JWT_ISSUER"), Some("from-file"));
}

#[test]
fn config_file_parser() {
    let ok_text = "\u{feff}# comment\n\nPHANTOM_BIND_ADDR=127.0.0.1:9000\nexport PHANTOM_JWT_ISSUER=\"quoted iss\"\nORIGIN_URL = http://127.0.0.1:9\n";
    let v = parse_config_file(ok_text).unwrap();
    assert!(v.contains(&("PHANTOM_JWT_ISSUER".to_string(), "quoted iss".to_string())));
    assert!(v.contains(&("ORIGIN_URL".to_string(), "http://127.0.0.1:9".to_string())));
    assert!(parse_config_file("BOGUS_KEY=1").is_err());
    assert!(parse_config_file("PHANTOM_SIMULATION=true\nPHANTOM_SIMULATION=false").is_err());
    assert!(parse_config_file("just text").is_err());
    assert!(parse_config_file("PHANTOM_CONFIG=other.conf").is_err());
    // all errors are reported, not only the first
    assert_eq!(parse_config_file("A=1\nB=2").unwrap_err().len(), 2);
}

#[test]
fn cli_parser() {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let c = parse_cli(&s(&["--config", "a.conf", "--bind=0.0.0.0:1", "--mode", "production", "--origin", "http://o", "--check-config"])).unwrap();
    assert_eq!(c.config.as_deref(), Some("a.conf"));
    assert_eq!(c.bind.as_deref(), Some("0.0.0.0:1"));
    assert_eq!(c.mode.as_deref(), Some("production"));
    assert!(c.check_config);
    assert!(parse_cli(&s(&["--bind"])).is_err());
    assert!(parse_cli(&s(&["--bind", "--mode"])).is_err());
    assert!(parse_cli(&s(&["--nope"])).is_err());
    assert!(parse_cli(&s(&["-h"])).unwrap().help);
    assert!(parse_cli(&s(&["-V"])).unwrap().version);
    let o = c.overrides();
    assert!(o.contains(&("PHANTOM_BIND_ADDR", "0.0.0.0:1".to_string())));
}

#[test]
fn secrets_never_appear_in_report_debug_errors_or_warnings() {
    let scenarios: Vec<Vec<(&str, &str)>> = vec![
        vec![("PHANTOM_JWT_SECRET", CANARY)],
        vec![("PHANTOM_JWT_SECRET", CANARY), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_BIND_ADDR", "bad")],
        vec![("PHANTOM_JWT_SECRET", CANARY), ("PHANTOM_MODE", "production"), ("PHANTOM_SIMULATION", "true")],
        vec![("PHANTOM_JWT_SECRET", CANARY), ("PHANTOM_BIND_ADDR", "0.0.0.0:1")],
    ];
    for s in scenarios {
        match resolve(&env(&s)) {
            Ok(c) => {
                let all = format!("{}\n{:?}\n{}", c.report(), c, c.warnings.join("\n"));
                assert!(!all.contains(CANARY), "secret leaked: {}", all);
                assert!(all.contains("<redacted>"));
                assert!(!format!("{:?}", c.env_exports()).is_empty());
            }
            Err(es) => {
                let all: String = es.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
                assert!(!all.contains(CANARY), "secret leaked in errors: {}", all);
            }
        }
    }
    // A too-short secret must be reported by length only.
    let short = "TopSecretShort";
    let es = err(&[("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", short)]);
    let all: String = es.iter().map(|e| e.to_string()).collect();
    assert!(!all.contains(short));
    assert!(all.contains("14 bytes"));
}

#[test]
fn resolution_is_idempotent_and_exports_round_trip() {
    let pairs = [
        ("PHANTOM_BIND_ADDR", " 127.0.0.1:9000 "),
        ("PHANTOM_SIMULATION", "TRUE"),
        ("PHANTOM_INTELLIGENCE", "Observe"),
        ("PHANTOM_ORIGIN_ALLOWLIST", "https://a.example.com , https://b.example.com,"),
    ];
    let a = ok(&pairs);
    let b = ok(&pairs);
    assert_eq!(a.report(), b.report());
    // Feed the normalized values back in: same result.
    let exported: BTreeMap<String, String> = a
        .env_exports()
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k.to_string(), v)))
        .collect();
    let mut again = Inputs { env: exported.clone(), ..Default::default() };
    again.env.insert("PHANTOM_INTELLIGENCE".into(), a.get("PHANTOM_INTELLIGENCE").unwrap().into());
    let c = resolve(&again).unwrap();
    for k in ["PHANTOM_BIND_ADDR", "PHANTOM_SIMULATION", "PHANTOM_INTELLIGENCE", "PHANTOM_ORIGIN_ALLOWLIST"] {
        assert_eq!(a.get(k), c.get(k), "{}", k);
    }
    assert_eq!(a.get("PHANTOM_ORIGIN_ALLOWLIST"), Some("https://a.example.com,https://b.example.com"));
}

#[test]
fn env_exports_semantics() {
    let i = Inputs {
        file: vec![("PHANTOM_BIND_ADDR".into(), "127.0.0.1:7777".into()), ("PHANTOM_STRICT_CLUSTER".into(), "false".into())],
        ..Default::default()
    };
    let ex = resolve(&i).unwrap().env_exports();
    assert!(ex.contains(&("PHANTOM_BIND_ADDR", Some("127.0.0.1:7777".to_string()))));
    assert!(ex.contains(&("PHANTOM_STRICT_CLUSTER", None)), "strict=false must remove the presence-based variable");
    assert!(ex.iter().all(|(k, _)| *k != "PHANTOM_MAX_STALE_MS" && *k != "PHANTOM_MODE"), "applied-by-main keys are not exported");
    assert!(resolve(&Inputs::default()).unwrap().env_exports().is_empty());
}

#[test]
fn unknown_phantom_env_warns_and_unrelated_is_ignored() {
    let c = ok(&[("PHANTOM_TYPO_KEY", "1")]);
    assert!(c.warnings.iter().any(|w| w.contains("PHANTOM_TYPO_KEY")));
    // env_snapshot only ever keeps PHANTOM_* and ORIGIN_URL
    assert!(env_snapshot().keys().all(|k| k.starts_with("PHANTOM_") || k == "ORIGIN_URL"));
}

#[test]
fn not_wired_default_ttl_is_reported_honestly() {
    let c = ok(&[("PHANTOM_DEFAULT_TTL_MS", "5000")]);
    assert!(c.warnings.iter().any(|w| w.contains("not honored")));
    assert!(c.report().contains("not honored by runtime"));
    assert!(!ok(&[]).warnings.iter().any(|w| w.contains("not honored")));
}

#[test]
fn preflight_checks_files() {
    let dir = std::env::temp_dir().join(format!("phantom_m7_pre_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("c.pem");
    std::fs::write(&f, "x").unwrap();
    let (c, k) = (f.to_str().unwrap().to_string(), f.to_str().unwrap().to_string());
    let good = ok(&[("PHANTOM_TLS_CERT", c.as_str()), ("PHANTOM_TLS_KEY", k.as_str())]);
    assert!(preflight(&good).0.is_empty());
    let missing = ok(&[("PHANTOM_TLS_CERT", "/definitely/not/here.pem"), ("PHANTOM_TLS_KEY", k.as_str())]);
    assert!(preflight(&missing).0.iter().any(|e| e.key.as_deref() == Some("PHANTOM_TLS_CERT")));
    let dirpath = dir.to_str().unwrap().to_string();
    let isdir = ok(&[("PHANTOM_TLS_CERT", dirpath.as_str()), ("PHANTOM_TLS_KEY", k.as_str())]);
    assert!(!preflight(&isdir).0.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn docs_and_code_list_the_same_keys() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/user/configuration.md"))
        .expect("docs/user/configuration.md must exist");
    let mut documented = BTreeSet::new();
    for line in doc.lines().filter(|l| l.starts_with("| `")) {
        let name = line.trim_start_matches("| `").split('`').next().unwrap();
        if name.starts_with("PHANTOM_") || name == "ORIGIN_URL" {
            documented.insert(name.to_string());
            if let Some(sp) = spec(name) {
                if let Some(d) = sp.default {
                    assert!(line.contains(d), "docs row for {} must show default {}", name, d);
                }
            }
        }
    }
    let coded: BTreeSet<String> = KEYS.iter().map(|k| k.name.to_string()).collect();
    assert_eq!(documented, coded, "docs/user/configuration.md and src/config.rs KEYS differ");
    // every flag in USAGE is documented
    for flag in ["--config", "--bind", "--mode", "--origin", "--check-config"] {
        assert!(doc.contains(flag), "flag {} undocumented", flag);
    }
}

// ---------------------------------------------------------------- SEC-M7-03 CORS configuration

#[test]
fn cors_origins_default_by_profile() {
    // Development profile (also the default): the Vite dev origins, explicitly.
    let dev = ok(&[]);
    assert_eq!(dev.get("PHANTOM_CORS_ORIGINS"), None, "the key itself stays unset; the profile default is derived");
    assert_eq!(dev.cors_origins(), vec!["http://localhost:3000".to_string(), "http://127.0.0.1:3000".to_string()]);
    assert_eq!(dev.cors_origins(), DEV_CORS_ORIGINS.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let vite = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/vite.config.js")).unwrap();
    assert!(vite.contains("port: 3000"), "DEV_CORS_ORIGINS must follow the Vite dev port");
    // Production profile: never a default allowlist (same-origin only), never permissive.
    let prod = ok(&[("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)]);
    assert!(prod.cors_origins().is_empty());
    // An explicit value replaces the default in both profiles.
    let c = ok(&[("PHANTOM_CORS_ORIGINS", "https://ui.example.com")]);
    assert_eq!(c.cors_origins(), vec!["https://ui.example.com".to_string()]);
    let p = ok(&[
        ("PHANTOM_MODE", "production"),
        ("PHANTOM_REQUIRE_AUTH", "true"),
        ("PHANTOM_JWT_SECRET", S32),
        ("PHANTOM_CORS_ORIGINS", "https://ui.example.com:8443"),
    ]);
    assert_eq!(p.cors_origins(), vec!["https://ui.example.com:8443".to_string()]);
}

#[test]
fn cors_origins_malformed_values_are_startup_errors_in_every_profile() {
    let prod = [("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)];
    for bad in [
        "*",
        "https://*.example.com",
        "https://ui.example.com/",
        "https://ui.example.com/app",
        "https://ui.example.com?x=1",
        "ftp://ui.example.com",
        "ui.example.com",
        "https://a.example.com,,https://b.example.com",
        "https://a.example.com,",
        "https://u:p@a.example.com",
    ] {
        let mut v = prod.to_vec();
        v.push(("PHANTOM_CORS_ORIGINS", bad));
        let e = err(&v);
        assert!(e.iter().any(|x| x.key.as_deref() == Some("PHANTOM_CORS_ORIGINS")), "{:?} must be rejected in production", bad);
        // same rule in development
        assert!(err(&[("PHANTOM_CORS_ORIGINS", bad)]).iter().any(|x| x.key.as_deref() == Some("PHANTOM_CORS_ORIGINS")), "{:?} dev", bad);
    }
    // errors aggregate with other errors
    let e = err(&[("PHANTOM_CORS_ORIGINS", "*"), ("PHANTOM_MAX_STALE_MS", "0")]);
    assert!(e.iter().any(|x| x.key.as_deref() == Some("PHANTOM_CORS_ORIGINS")));
    assert!(e.iter().any(|x| x.key.as_deref() == Some("PHANTOM_MAX_STALE_MS")));
    // more than 16 entries
    let many = (0..17).map(|i| format!("https://h{}.example.com", i)).collect::<Vec<_>>().join(",");
    assert!(!err(&[("PHANTOM_CORS_ORIGINS", many.as_str())]).is_empty());
}

#[test]
fn cors_origins_are_normalized_and_reported() {
    let c = ok(&[("PHANTOM_CORS_ORIGINS", " HTTPS://UI.Example.com:443 , http://localhost:3000 ,https://ui.example.com")]);
    assert_eq!(c.get("PHANTOM_CORS_ORIGINS"), Some("https://ui.example.com,http://localhost:3000"));
    let report = c.report();
    assert!(report.contains("PHANTOM_CORS_ORIGINS"));
    assert!(report.contains("effective CORS origins: https://ui.example.com,http://localhost:3000"));
    assert!(report.contains("not authentication"));
    let prod = ok(&[("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", S32)]);
    assert!(prod.report().contains("effective CORS origins: none (same-origin only)"));
}

#[test]
fn cors_origins_set_but_empty_means_no_cross_origin_access_even_in_development() {
    // unset keeps the profile default ...
    assert!(!ok(&[]).cors_origins().is_empty());
    // ... but an explicitly set, empty value is "no cross-origin access" (round-2 review).
    {
        let pairs = vec![("PHANTOM_CORS_ORIGINS", "")];
        let c = ok(&pairs);
        assert!(c.cors_origins().is_empty());
        assert_eq!(c.get("PHANTOM_CORS_ORIGINS"), Some(""));
        assert!(c.report().contains("effective CORS origins: none (same-origin only)"));
    }
}

#[test]
fn cors_origins_errors_redact_userinfo_and_dev_defaults_warn_on_network_binds() {
    let e = err(&[("PHANTOM_CORS_ORIGINS", "https://user:s3cretpw@ui.example.com")]);
    let text = e.iter().map(|x| x.to_string()).collect::<String>();
    assert!(text.contains("PHANTOM_CORS_ORIGINS") && !text.contains("s3cretpw") && !text.contains("user:"), "{}", text);
    assert!(parse_cors_origins("http://a@b.com").unwrap_err().find("a@b").is_none());
    // dev defaults on a non-loopback bind -> warning; explicit value or empty value -> none
    let net = [("PHANTOM_BIND_ADDR", "0.0.0.0:8443"), ("PHANTOM_JWT_SECRET", S32), ("PHANTOM_TLS_CERT", "c.pem"), ("PHANTOM_TLS_KEY", "k.pem")];
    let c = ok(&net);
    assert!(c.warnings.iter().any(|w| w.contains("PHANTOM_CORS_ORIGINS") && w.contains("development")), "{:?}", c.warnings);
    for extra in ["https://ui.example.com", ""] {
        let mut v = net.to_vec();
        v.push(("PHANTOM_CORS_ORIGINS", extra));
        assert!(!ok(&v).warnings.iter().any(|w| w.contains("development profile CORS")), "{:?}", extra);
    }
    assert!(!ok(&[]).warnings.iter().any(|w| w.contains("development profile CORS")), "loopback default has no warning");
}
