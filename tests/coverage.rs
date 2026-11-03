use multi_pal_prime::{
    BigSpace, Space, atomic_write, binary_pal, binary_pal_big, decimal_pal, decimal_pal_big,
    hex_pal, hex_pal_big, reference_pal,
};
use num_bigint::BigUint;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn integration_small_spaces_match_brute_force() {
    for bits in 2..=18 {
        let s = Space::new(bits);
        let actual = (0..s.count())
            .map(|i| s.candidate(i))
            .collect::<std::collections::BTreeSet<_>>();
        let lo = 1u128 << (bits - 1);
        let hi = 1u128 << bits;
        let expected = (lo..hi)
            .filter(|&n| reference_pal(n, 2) && reference_pal(n, 16))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected, "length {bits}");
    }
}

#[test]
fn fixed_seed_property_palindrome_checks() {
    let mut x = 0x9e3779b97f4a7c15u128;
    for _ in 0..5000 {
        x ^= x << 23;
        x ^= x >> 17;
        x ^= x << 29;
        let n = x.max(1);
        assert_eq!(binary_pal(n), reference_pal(n, 2));
        assert_eq!(hex_pal(n), reference_pal(n, 16));
        assert_eq!(decimal_pal(n), reference_pal(n, 10));
    }
}

#[test]
fn all_bit_lengths_and_hex_length_transitions_are_covered() {
    for bits in 2..=128 {
        let s = Space::new(bits);
        let count = s.count();
        let mut samples = vec![0, count - 1, count / 2];
        let mut x = (bits as u128).wrapping_mul(0xd6e8feb86659fd93);
        for _ in 0..8 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            samples.push(x % count);
        }
        for a in samples {
            let n = s.candidate(a);
            assert_eq!(128 - n.leading_zeros(), bits);
            assert!(reference_pal(n, 2));
            assert!(reference_pal(n, 16));
        }
    }
}

#[test]
fn bigint_generator_matches_reference_and_scales_to_256_bits() {
    for bits in 2..=20 {
        let s = BigSpace::new(bits);
        let mut actual = std::collections::BTreeSet::new();
        for a in 0..s.count().to_u64_digits().first().copied().unwrap_or(0) {
            let n = s.candidate(&BigUint::from(a));
            assert!(binary_pal_big(&n));
            assert!(hex_pal_big(&n));
            let gray_candidate = s.gray_start(&BigUint::from(a));
            assert_eq!(s.assignment_index(&gray_candidate), Some(BigUint::from(a)));
            actual.insert(n);
        }
        let lo = 1u128 << (bits - 1);
        let hi = 1u128 << bits;
        let expected = (lo..hi)
            .filter(|&n| reference_pal(n, 2) && reference_pal(n, 16))
            .map(BigUint::from)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected, "bits={bits}");
    }
    for bits in [127u32, 128, 129, 191, 192, 255, 256] {
        let s = BigSpace::new(bits);
        let count = s.count();
        for a in [
            BigUint::from(0u8),
            &count - BigUint::from(1u8),
            &count / BigUint::from(2u8),
        ] {
            let n = s.candidate(&a);
            assert_eq!(n.bits(), bits as u64);
            assert!(binary_pal_big(&n));
            assert!(hex_pal_big(&n));
            assert!(s.assignment_index(&n).is_some());
        }
    }
    for n in [3u32, 5, 7, 9, 10, 11, 121] {
        let n = BigUint::from(n);
        assert_eq!(
            decimal_pal_big(&n),
            reference_pal(n.to_string().parse::<u128>().unwrap(), 10)
        );
    }
}

#[test]
fn chunk_boundaries_and_recovery_are_idempotent() {
    let s = Space::new(35);
    let chunk = 200u128;
    let mut a = (0..chunk).map(|i| s.candidate(i)).collect::<Vec<_>>();
    let mut b = (chunk..2 * chunk)
        .map(|i| s.candidate(i))
        .collect::<Vec<_>>();
    a.append(&mut b);
    let whole = (0..2 * chunk).map(|i| s.candidate(i)).collect::<Vec<_>>();
    assert_eq!(a, whole);
    let mut gray = s.gray_start(73);
    let mut gray_values = Vec::new();
    for i in 73..273 {
        gray_values.push(gray);
        if i + 1 < 273 {
            gray ^= s.gray_flip_mask(i + 1);
        }
    }
    let direct = (73..273)
        .map(|i| s.candidate(i ^ (i >> 1)))
        .collect::<Vec<_>>();
    assert_eq!(gray_values, direct);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = PathBuf::from(format!("/tmp/mpal-checkpoint-{nonce}.json"));
    atomic_write(&p, b"committed checkpoint\n").unwrap();
    assert_eq!(fs::read(&p).unwrap(), b"committed checkpoint\n");
    atomic_write(&p, b"replacement checkpoint\n").unwrap();
    assert_eq!(fs::read(&p).unwrap(), b"replacement checkpoint\n");
    fs::remove_file(p).unwrap();
    let relative = PathBuf::from(format!(".mpal-relative-checkpoint-{nonce}.json"));
    atomic_write(&relative, b"relative destination\n").unwrap();
    assert_eq!(fs::read(&relative).unwrap(), b"relative destination\n");
    fs::remove_file(relative).unwrap();
}

#[test]
fn verifier_detects_missing_or_malformed_partition_records() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = PathBuf::from(format!("/tmp/mpal-verify-count-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    // The b=2 candidate space has a single assignment. The footer claims one
    // decimal palindrome result but the row is absent, reproducing truncation.
    fs::write(
        root.join(format!("part-b2-a{:039}.jsonl", 0)),
        "{\"status\":\"completed\",\"bits\":2,\"assignment_start\":\"0\",\"assignment_end_exclusive\":\"1\",\"assignment_space\":\"1\",\"generated\":1,\"decimal_pass\":1}\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_multi-pal-prime"))
        .arg("verify")
        .arg(&root)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("footer_result_count=1"), "{stdout}");
    assert!(stdout.contains("parsed_records=0"), "{stdout}");
    assert!(stdout.contains("invalid_partitions=1"), "{stdout}");
    assert!(stdout.contains("complete=false"), "{stdout}");

    let report = Command::new(env!("CARGO_BIN_EXE_multi-pal-prime"))
        .arg("report")
        .arg(&root)
        .output()
        .unwrap();
    assert!(report.status.success());
    let report = String::from_utf8(report.stdout).unwrap();
    assert!(report.contains("\"record_audit_passed\":false"), "{report}");
    assert!(
        report.contains("\"status\":\"incomplete_or_unresolved\""),
        "{report}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bigint_index_cli_matches_generator_and_rejects_nonmembers() {
    let out = Command::new(env!("CARGO_BIN_EXE_multi-pal-prime"))
        .args(["big-index", "4", "9"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap().trim(), "0");

    let out = Command::new(env!("CARGO_BIN_EXE_multi-pal-prime"))
        .args(["big-index", "4", "3"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
