use phantom_cache_demo::intelligence::audit::*;
use phantom_cache_demo::intelligence::*;

fn make_record(id: u64, verdict: Verdict) -> DecisionRecord {
    DecisionRecord {
        decision_id: id,
        timestamp_ms: 1000 + id * 10,
        mode: IntelligenceMode::Canary,
        trigger_reason: "Periodic evaluation".to_string(),
        candidate: PolicyCandidate {
            candidate_id: format!("CAND_{}", id),
            kind: PolicyKind::Eviction,
            parameters: [("p".to_string(), 1.0)].into_iter().collect(),
            bounds: [("p".to_string(), (0.0, 2.0))].into_iter().collect(),
            expected_effect: "Optimization".to_string(),
        },
        estimated_effect: 0.12,
        confidence_interval: (0.06, 0.18),
        guardrails_passed: verdict != Verdict::Rollback,
        verdict,
        explanation: format!("Evaluation result for decision {}", id),
        config_hash_before: "CONF_PREV".to_string(),
        config_hash_after: format!("CONF_NEW_{}", id),
    }
}

#[test]
fn test_audit_store_bounded_ring_buffer() {
    let mut store = AuditStore::new(100);

    for i in 1..=150 {
        store.append(make_record(i, Verdict::Commit));
    }

    assert_eq!(store.len(), 100);
    assert_eq!(store.total_recorded(), 150);

    // Oldest surviving record should be decision_id 51
    assert!(store.get_by_id(1).is_none());
    assert!(store.get_by_id(50).is_none());
    assert!(store.get_by_id(51).is_some());
    assert!(store.get_by_id(150).is_some());
}

#[test]
fn test_audit_store_query_by_verdict() {
    let mut store = AuditStore::new(50);

    for i in 1..=10 {
        store.append(make_record(i, Verdict::Commit));
    }
    for i in 11..=15 {
        store.append(make_record(i, Verdict::Rollback));
    }
    for i in 16..=20 {
        store.append(make_record(i, Verdict::NoAction));
    }

    assert_eq!(store.decisions_by_verdict(Verdict::Commit).len(), 10);
    assert_eq!(store.decisions_by_verdict(Verdict::Rollback).len(), 5);
    assert_eq!(store.decisions_by_verdict(Verdict::NoAction).len(), 5);
}

#[test]
fn test_audit_store_json_export_import_roundtrip() {
    let mut store = AuditStore::new(20);
    for i in 1..=5 {
        store.append(make_record(i, Verdict::Commit));
    }

    let json = store.export_json().expect("Export failed");
    let imported = AuditStore::import_json(&json).expect("Import failed");

    assert_eq!(imported.len(), store.len());
    assert_eq!(imported.total_recorded(), store.total_recorded());

    for i in 1..=5 {
        let orig = store.get_by_id(i).unwrap();
        let imp = imported.get_by_id(i).unwrap();
        assert_eq!(orig.decision_id, imp.decision_id);
        assert_eq!(orig.verdict, imp.verdict);
        assert_eq!(orig.candidate.candidate_id, imp.candidate.candidate_id);
    }
}

#[test]
fn test_audit_store_explainability_reconstructability() {
    let record = make_record(42, Verdict::Commit);
    let explanation = AuditStore::explain_decision(&record);

    assert!(explanation.contains("Decision #42 [Commit]"));
    assert!(explanation.contains("Candidate: CAND_42"));
    assert!(explanation.contains("Estimated Effect: 0.1200"));
    assert!(explanation.contains("95% CI: [0.0600, 0.1800]"));
    assert!(explanation.contains("Guardrails Passed: true"));
    assert!(explanation.contains("CONF_PREV -> CONF_NEW_42"));
}

