//! Versioned HTTP retry-hint vectors consumed by client and delivery adapters.
//! This repository publishes the wire fixture, not a production retry scheduler.

use std::collections::BTreeSet;

#[test]
fn retry_after_delta_seconds_vectors_are_deterministic_and_bounded() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/retry_after_delta_seconds.json"))
            .expect("valid reference JSON");
    assert_eq!(fixture["version"], "eal.retry-after.delta-seconds.v1");
    let max_delay = fixture["maxDelaySeconds"]
        .as_u64()
        .expect("bounded maximum");
    assert_eq!(max_delay, 3600);
    let cases = fixture["cases"].as_array().expect("versioned vectors");
    assert!(cases.len() >= 10);

    let mut seen = BTreeSet::new();
    for case in cases {
        let raw = case["header"].as_str().expect("HTTP header string");
        assert!(seen.insert(raw), "duplicate test vector: {raw:?}");
        let trimmed = raw.trim();
        let expected = if !trimmed.is_empty() && trimmed.bytes().all(|b| b.is_ascii_digit()) {
            trimmed.parse::<u64>().ok().map(|seconds| seconds.min(max_delay))
        } else {
            None
        };
        assert_eq!(
            case["delaySeconds"].as_u64(),
            expected,
            "inconsistent retry-hint vector: {raw:?}"
        );
    }
}
