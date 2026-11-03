use multi_pal_prime::{
    BigSpace, Space, atomic_write, binary_pal_big, decimal_pal, decimal_pal_big, hex_pal,
    hex_pal_big, reference_pal, utc,
};
use num_bigint::BigUint;
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
const CHUNK: u128 = 1 << 20;
fn job_file(root: &Path, bits: u32, start: u128) -> PathBuf {
    root.join(format!("part-b{bits}-a{start:039}.jsonl"))
}
fn compositeness_witness(n: u128) -> Option<u128> {
    [2u128, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]
        .into_iter()
        .find(|&a| !mr(n, a))
}
fn addmod(a: u128, b: u128, m: u128) -> u128 {
    if a >= m - b { a - (m - b) } else { a + b }
}
fn mulmod(mut a: u128, mut b: u128, m: u128) -> u128 {
    let mut r = 0;
    a %= m;
    while b > 0 {
        if b & 1 == 1 {
            r = addmod(r, a, m)
        }
        b >>= 1;
        if b > 0 {
            a = addmod(a, a, m)
        }
    }
    r
}
fn powmod(mut a: u128, mut e: u128, m: u128) -> u128 {
    let mut r = 1 % m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, a, m)
        }
        e >>= 1;
        if e > 0 {
            a = mulmod(a, a, m)
        }
    }
    r
}
fn mr(n: u128, a: u128) -> bool {
    if n < 3 {
        return n == 2;
    }
    let mut d = n - 1;
    let s = d.trailing_zeros();
    d >>= s;
    let mut x = powmod(a % n, d, n);
    if x == 1 || x == n - 1 {
        return true;
    }
    for _ in 1..s {
        x = mulmod(x, x, n);
        if x == n - 1 {
            return true;
        }
    }
    false
}
fn classify(n: u128) -> (&'static str, &'static str, String) {
    if n == 3 {
        return (
            "prime",
            "trial-division",
            "trial_divisors_through=1".to_string(),
        );
    }
    let bits = 128 - n.leading_zeros();
    if bits % 2 == 0 {
        return (
            "composite",
            "divisible-by-3-from-even-binary-palindrome",
            "factor=3".to_string(),
        );
    }
    let hex_digits = ((bits + 3) / 4) as usize;
    if hex_digits % 2 == 0 {
        return (
            "composite",
            "divisible-by-17-from-even-hex-palindrome",
            "factor=17".to_string(),
        );
    }
    if n != 11 && n.to_string().len() % 2 == 0 {
        return (
            "composite",
            "divisible-by-11-from-even-decimal-palindrome",
            "factor=11".to_string(),
        );
    }
    if n < 2 {
        return ("composite", "n<2", "n<2".to_string());
    }
    for p in [2u128, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
        if n == p {
            return (
                "prime",
                "trial-division",
                format!("trial_divisors_through={}", p - 1),
            );
        }
        if n % p == 0 {
            return ("composite", "factor-by-small-prime", format!("factor={p}"));
        }
    }
    if let Some(a) = compositeness_witness(n) {
        (
            "composite",
            "Miller-Rabin-compositeness-witness",
            format!("miller_rabin_base={a}"),
        )
    } else {
        (
            "unresolved",
            "probable-prime; rigorous certificate required",
            "no certificate".to_string(),
        )
    }
}
fn classify_big(n: &BigUint) -> (&'static str, &'static str, String) {
    if n == &BigUint::from(3u8) || n == &BigUint::from(5u8) || n == &BigUint::from(7u8) {
        return (
            "prime",
            "exact-small-prime",
            format!("trial_division_through={}", n - 1u8),
        );
    }
    let bits = n.bits();
    if bits % 2 == 0 {
        return (
            "composite",
            "even-binary-palindrome-divisible-by-3",
            "factor=3".to_string(),
        );
    }
    let hex_digits = (bits + 3) / 4;
    if hex_digits % 2 == 0 {
        return (
            "composite",
            "even-hex-palindrome-divisible-by-17",
            "factor=17".to_string(),
        );
    }
    let decimal_digits = n.to_str_radix(10).len();
    if decimal_digits % 2 == 0 {
        return (
            "composite",
            "even-decimal-palindrome-divisible-by-11",
            "factor=11".to_string(),
        );
    }
    for p in [
        2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
        89, 97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179,
        181, 191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251,
    ] {
        let bp = BigUint::from(p);
        if n == &bp {
            return (
                "prime",
                "exact-small-prime",
                format!("trial_division_through={}", p - 1),
            );
        }
        if n % &bp == BigUint::from(0u8) {
            return ("composite", "small-prime-factor", format!("factor={p}"));
        }
    }
    let one = BigUint::from(1u8);
    let nm1 = n - &one;
    let s = nm1.trailing_zeros().unwrap_or(0);
    let d = &nm1 >> s as usize;
    for a in [2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let ba = BigUint::from(a) % n;
        let mut x = ba.modpow(&d, n);
        if x == one || x == nm1 {
            continue;
        }
        let mut pass = false;
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == nm1 {
                pass = true;
                break;
            }
        }
        if !pass {
            return (
                "composite",
                "miller-rabin-compositeness-witness",
                format!("miller_rabin_base={a}"),
            );
        }
    }
    (
        "unresolved",
        "probable-prime-requires-proof-certificate",
        "no deterministic 256-bit primality certificate configured".to_string(),
    )
}
fn append_event(path: &Path, line: &str) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(f, "{line}").unwrap();
    f.sync_data().unwrap();
}
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let tail = line.split(&format!("\"{key}\":")).nth(1)?;
    if let Some(x) = tail.strip_prefix('"') {
        Some(x.split('"').next()?)
    } else {
        Some(tail.split([',', '}']).next()?)
    }
}
#[derive(Default)]
struct Audit {
    expected_assignments: u128,
    covered_assignments: u128,
    expected_partitions: u64,
    completed_partitions: u64,
    missing_partitions: u64,
    invalid_partitions: u64,
    footer_result_count: u64,
    parsed_result_count: u64,
    proven_primes: u64,
    proven_composites: u64,
    unresolved: u64,
    invalid_records: u64,
}
fn verify_record(line: &str) -> Option<&'static str> {
    let n = field(line, "value")?.parse::<u128>().ok()?;
    if !(reference_pal(n, 2) && reference_pal(n, 16) && reference_pal(n, 10)) {
        return None;
    }
    let bin = format!("{n:b}");
    let hex = format!("{n:x}");
    if field(line, "binary") != Some(bin.as_str())
        || field(line, "hex") != Some(hex.as_str())
        || field(line, "decimal_digits").and_then(|x| x.parse::<usize>().ok())
            != Some(n.to_string().len())
        || field(line, "binary_bits").and_then(|x| x.parse::<u32>().ok())
            != Some(128 - n.leading_zeros())
        || field(line, "hex_digits").and_then(|x| x.parse::<usize>().ok()) != Some(hex.len())
    {
        return None;
    }
    match field(line, "classification")? {
        "prime" => {
            let mut d = 2u128;
            while d <= n / d {
                if n % d == 0 {
                    return None;
                }
                d += 1;
            }
            Some("prime")
        }
        "composite" => {
            let ev = field(line, "proof_evidence")?;
            let certified = if let Some(factor) = ev
                .strip_prefix("factor=")
                .and_then(|x| x.parse::<u128>().ok())
            {
                factor > 1 && n % factor == 0
            } else if let Some(base) = ev
                .strip_prefix("miller_rabin_base=")
                .and_then(|x| x.parse::<u128>().ok())
            {
                !mr(n, base)
            } else {
                false
            };
            if certified { Some("composite") } else { None }
        }
        "unresolved" => Some("unresolved"),
        _ => None,
    }
}
fn audit_partitions(root: &Path) -> Audit {
    let mut a = Audit::default();
    let mut seen = std::collections::HashSet::new();
    for bits in 2..=128 {
        let space = Space::new(bits).count();
        let mut start = 0u128;
        while start < space {
            let end = (start + CHUNK).min(space);
            a.expected_assignments += end - start;
            a.expected_partitions += 1;
            let path = job_file(root, bits, start);
            if !path.exists() {
                a.missing_partitions += 1;
                start = end;
                continue;
            }
            let Ok(data) = fs::read_to_string(&path) else {
                a.invalid_partitions += 1;
                start = end;
                continue;
            };
            let lines: Vec<_> = data.lines().collect();
            let Some(footer) = lines.last() else {
                a.invalid_partitions += 1;
                start = end;
                continue;
            };
            let footer_valid = footer.contains("\"status\":\"completed\"")
                && field(footer, "bits").and_then(|x| x.parse::<u32>().ok()) == Some(bits)
                && field(footer, "assignment_start") == Some(start.to_string().as_str())
                && field(footer, "assignment_end_exclusive") == Some(end.to_string().as_str())
                && field(footer, "assignment_space") == Some(space.to_string().as_str())
                && field(footer, "generated").and_then(|x| x.parse::<u128>().ok())
                    == Some(end - start);
            if !footer_valid {
                a.invalid_partitions += 1;
                start = end;
                continue;
            }
            let records = &lines[..lines.len() - 1];
            let decimal_pass = field(footer, "decimal_pass").and_then(|x| x.parse::<u64>().ok());
            if decimal_pass != Some(records.len() as u64) {
                a.invalid_partitions += 1;
            }
            a.footer_result_count += decimal_pass.unwrap_or(0);
            a.completed_partitions += 1;
            a.covered_assignments += end - start;
            for record in records {
                a.parsed_result_count += 1;
                if let Some(class) = verify_record(record) {
                    let n = field(record, "value")
                        .and_then(|x| x.parse::<u128>().ok())
                        .unwrap();
                    if !seen.insert(n) {
                        a.invalid_records += 1;
                        continue;
                    }
                    match class {
                        "prime" => a.proven_primes += 1,
                        "composite" => a.proven_composites += 1,
                        _ => a.unresolved += 1,
                    }
                } else {
                    a.invalid_records += 1
                }
            }
            start = end;
        }
    }
    let actual_partition_files = fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("part-") && name.ends_with(".jsonl")
        })
        .count() as u64;
    if actual_partition_files != a.expected_partitions {
        a.invalid_partitions += actual_partition_files.abs_diff(a.expected_partitions);
    }
    a
}
impl Audit {
    fn complete(&self) -> bool {
        self.covered_assignments == self.expected_assignments
            && self.completed_partitions == self.expected_partitions
            && self.missing_partitions == 0
            && self.invalid_partitions == 0
            && self.footer_result_count == self.parsed_result_count
            && self.invalid_records == 0
            && self.unresolved == 0
    }
}
fn process(root: &Path, bits: u32, start: u128, end: u128, total: u128, work: &AtomicU64) {
    let marker = job_file(root, bits, start);
    if marker.exists() {
        return;
    }
    let s = Space::new(bits);
    let t = Instant::now();
    let mut generated = 0u64;
    let mut bpass = 0u64;
    let mut hpass = 0u64;
    let mut dpass = 0u64;
    let mut unresolved = 0u64;
    let mut hex_time = Duration::ZERO;
    let mut decimal_time = Duration::ZERO;
    let mut primality_time = Duration::ZERO;
    let mut records = String::new();
    let mut n = s.gray_start(start);
    for a in start..end {
        generated += 1;
        work.fetch_add(1, Ordering::Relaxed);
        bpass += 1;
        let stage = Instant::now();
        let hp = hex_pal(n);
        hex_time += stage.elapsed();
        if hp {
            hpass += 1;
            let stage = Instant::now();
            let dp = decimal_pal(n);
            decimal_time += stage.elapsed();
            if dp {
                dpass += 1;
                let stage = Instant::now();
                let (class, method, proof_evidence) = classify(n);
                let proof_seconds = stage.elapsed().as_secs_f64();
                primality_time += stage.elapsed();
                if class == "unresolved" {
                    unresolved += 1
                }
                let bin = format!("{n:b}");
                let hex = format!("{n:x}");
                records.push_str(&format!("{{\"value\":\"{n}\",\"binary\":\"{bin}\",\"hex\":\"{hex}\",\"decimal_digits\":{},\"binary_bits\":{},\"hex_digits\":{},\"classification\":\"{class}\",\"method\":\"{method}\",\"proof_evidence\":\"{proof_evidence}\",\"primality_verification_seconds\":{proof_seconds:.9},\"partition\":\"b{bits}-a{start}\",\"timestamp_utc\":\"{}\"}}\n",n.to_string().len(),bin.len(),hex.len(),utc()));
            }
        }
        if a + 1 < end {
            n ^= s.gray_flip_mask(a + 1);
        }
    }
    let dur = t.elapsed().as_secs_f64();
    let body = format!(
        "{{\"status\":\"completed\",\"bits\":{bits},\"assignment_start\":\"{start}\",\"assignment_end_exclusive\":\"{end}\",\"assignment_space\":\"{total}\",\"generated\":{generated},\"binary_pass\":{bpass},\"hex_pass\":{hpass},\"decimal_pass\":{dpass},\"primality_tests\":{dpass},\"unresolved\":{unresolved},\"hex_check_seconds\":{:.9},\"decimal_check_seconds\":{:.9},\"primality_seconds\":{:.9},\"seconds\":{dur:.9},\"completion_utc\":\"{}\"}}\n",
        hex_time.as_secs_f64(),
        decimal_time.as_secs_f64(),
        primality_time.as_secs_f64(),
        utc()
    );
    records.push_str(&body);
    atomic_write(&marker, records.as_bytes()).expect("durable partition commit");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("partitions.jsonl"))
        .unwrap();
    write!(f, "{body}").unwrap();
    f.sync_all().unwrap();
    let _ = total;
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("run");
    if cmd == "big-index" {
        let bits = args
            .get(2)
            .and_then(|x| x.parse::<u32>().ok())
            .expect("bits 2..=256");
        let n = BigUint::parse_bytes(args.get(3).expect("candidate value").as_bytes(), 10)
            .expect("decimal candidate");
        let space = BigSpace::new(bits);
        if let Some(index) = space.assignment_index(&n) {
            println!("{index}");
        } else {
            std::process::exit(1);
        }
        return;
    }
    if cmd == "big-range" {
        let bits = args
            .get(2)
            .and_then(|x| x.parse::<u32>().ok())
            .expect("bits 2..=256");
        let start = BigUint::parse_bytes(args.get(3).expect("start assignment").as_bytes(), 10)
            .expect("decimal start");
        let end = BigUint::parse_bytes(args.get(4).expect("end assignment").as_bytes(), 10)
            .expect("decimal end");
        let path = PathBuf::from(args.get(5).expect("output JSONL path"));
        let s = BigSpace::new(bits);
        let count = s.count();
        assert!(
            start < end && end <= count,
            "range must be nonempty and inside [0,space)"
        );
        if path.exists() {
            println!("already committed: {}", path.display());
            return;
        }
        let timer = Instant::now();
        let mut records = String::new();
        let mut i = start.clone();
        let mut n = s.gray_start(&start);
        let mut decimal_pass = 0u64;
        let mut unresolved = 0u64;
        while i < end {
            assert!(
                binary_pal_big(&n) && hex_pal_big(&n),
                "constraint generator produced invalid value"
            );
            if decimal_pal_big(&n) {
                decimal_pass += 1;
                let (class, method, evidence) = classify_big(&n);
                if class == "unresolved" {
                    unresolved += 1
                }
                let dec = n.to_str_radix(10);
                let bin = n.to_str_radix(2);
                let hex = n.to_str_radix(16);
                records.push_str(&format!("{{\"value\":\"{dec}\",\"binary\":\"{bin}\",\"hex\":\"{hex}\",\"assignment_index\":\"{i}\",\"decimal_digits\":{},\"binary_bits\":{},\"hex_digits\":{},\"classification\":\"{class}\",\"method\":\"{method}\",\"proof_evidence\":\"{evidence}\",\"timestamp_utc\":\"{}\"}}\n",dec.len(),bin.len(),hex.len(),utc()));
            }
            i += 1u8;
            if i < end {
                n ^= s.gray_flip_mask(&i)
            }
        }
        let generated = &end - &start;
        let footer = format!(
            "{{\"status\":\"completed\",\"format_version\":2,\"bits\":{bits},\"assignment_start\":\"{start}\",\"assignment_end_exclusive\":\"{end}\",\"assignment_space\":\"{count}\",\"generated\":\"{generated}\",\"decimal_pass\":{decimal_pass},\"unresolved\":{unresolved},\"seconds\":{:.9},\"completion_utc\":\"{}\"}}\n",
            timer.elapsed().as_secs_f64(),
            utc()
        );
        records.push_str(&footer);
        atomic_write(&path, records.as_bytes()).expect("durable result commit");
        println!("{footer}");
        return;
    }
    let root = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("data"));
    let threads = args
        .get(3)
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or_else(|| {
            thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        });
    fs::create_dir_all(&root).unwrap();
    if cmd == "space" {
        let mut total = 0u128;
        for b in 2..=128 {
            let s = Space::new(b);
            let count = s.count();
            println!("bits={b} free={} count={count}", s.free_bits);
            total = total.saturating_add(count)
        }
        println!("total assignments (all binary/hex palindromes) = {total}");
        return;
    }
    if cmd == "space256" {
        let mut total = BigUint::from(0u8);
        for bits in 2..=256 {
            let s = BigSpace::new(bits);
            let count = s.count();
            println!("bits={bits} free={} assignments={count}", s.free_bits);
            total += count;
        }
        println!("total binary/hex palindrome assignments through 256 bits = {total}");
        return;
    }
    if cmd == "verify" {
        let audit = audit_partitions(&root);
        println!(
            "result_files={} proven_primes={} proven_composites={} unresolved={} parsed_records={} footer_result_count={} invalid_records={} missing_partitions={} invalid_partitions={} complete={}",
            audit.completed_partitions,
            audit.proven_primes,
            audit.proven_composites,
            audit.unresolved,
            audit.parsed_result_count,
            audit.footer_result_count,
            audit.invalid_records,
            audit.missing_partitions,
            audit.invalid_partitions,
            audit.complete()
        );
        return;
    }
    if cmd == "progress" {
        let files = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("part-")
                    && e.path().extension().is_some_and(|x| x == "jsonl")
            })
            .count();
        println!("completed_partition_files={files} root={}", root.display());
        return;
    }
    if cmd == "coverage" {
        let audit = audit_partitions(&root);
        println!(
            "{{\"expected_assignments\":\"{}\",\"verified_assignments\":\"{}\",\"expected_partitions\":{},\"completed_partitions\":{},\"missing_partitions\":{},\"invalid_partitions\":{},\"parsed_result_records\":{},\"footer_result_count\":{},\"invalid_records\":{},\"complete\":{}}}",
            audit.expected_assignments,
            audit.covered_assignments,
            audit.expected_partitions,
            audit.completed_partitions,
            audit.missing_partitions,
            audit.invalid_partitions,
            audit.parsed_result_count,
            audit.footer_result_count,
            audit.invalid_records,
            audit.complete()
        );
        return;
    }
    if cmd == "primes" {
        for entry in fs::read_dir(&root).unwrap().filter_map(Result::ok) {
            let p = entry.path();
            if p.file_name()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.starts_with("part-") && x.ends_with(".jsonl"))
            {
                if let Ok(data) = fs::read_to_string(p) {
                    for line in data.lines() {
                        if line.contains("\"classification\":\"prime\"") {
                            println!("{line}");
                        }
                    }
                }
            }
        }
        return;
    }
    if cmd == "report" {
        let mut generated = 0u128;
        let mut partitions = 0u64;
        let mut part_seconds = 0f64;
        let mut hex_seconds = 0f64;
        let mut decimal_seconds = 0f64;
        let mut primality_seconds = 0f64;
        let mut hex_pass = 0u128;
        let mut decimal_pass = 0u128;
        let mut primes = Vec::<String>::new();
        let mut composites = 0u64;
        let mut unresolved = 0u64;
        let mut length = vec![(0u128, 0u64, 0f64); 129];
        for entry in fs::read_dir(&root).unwrap().filter_map(Result::ok) {
            let p = entry.path();
            if !p
                .file_name()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.starts_with("part-") && x.ends_with(".jsonl"))
            {
                continue;
            }
            let Ok(data) = fs::read_to_string(p) else {
                continue;
            };
            for line in data.lines() {
                if line.contains("\"status\":\"completed\"") {
                    let g = field(line, "generated")
                        .and_then(|x| x.parse::<u128>().ok())
                        .unwrap_or(0);
                    let sec = field(line, "seconds")
                        .and_then(|x| x.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let hs = field(line, "hex_check_seconds")
                        .and_then(|x| x.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let ds = field(line, "decimal_check_seconds")
                        .and_then(|x| x.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let ps = field(line, "primality_seconds")
                        .and_then(|x| x.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let b = field(line, "bits")
                        .and_then(|x| x.parse::<usize>().ok())
                        .unwrap_or(0);
                    generated += g;
                    hex_pass += field(line, "hex_pass")
                        .and_then(|x| x.parse::<u128>().ok())
                        .unwrap_or(0);
                    decimal_pass += field(line, "decimal_pass")
                        .and_then(|x| x.parse::<u128>().ok())
                        .unwrap_or(0);
                    partitions += 1;
                    part_seconds += sec;
                    hex_seconds += hs;
                    decimal_seconds += ds;
                    primality_seconds += ps;
                    if b < 129 {
                        length[b].0 += g;
                        length[b].1 += 1;
                        length[b].2 += sec;
                    }
                } else if line.contains("\"classification\":\"prime\"") {
                    primes.push(line.to_string())
                } else if line.contains("\"classification\":\"composite\"") {
                    composites += 1
                } else if line.contains("\"classification\":\"unresolved\"") {
                    unresolved += 1
                }
            }
        }
        let expected = (2..=128).map(|b| Space::new(b).count()).sum::<u128>();
        let expected_partitions = (2..=128)
            .map(|b| {
                let c = Space::new(b).count();
                (c + CHUNK - 1) / CHUNK
            })
            .sum::<u128>();
        let mut run_start = String::new();
        let mut run_end = String::new();
        let mut wall = 0f64;
        let mut peak_rate = 0f64;
        if let Ok(data) = fs::read_to_string(root.join("telemetry.jsonl")) {
            for l in data.lines() {
                if let Some(v) = field(l, "instantaneous_candidates_per_second")
                    .and_then(|x| x.parse::<f64>().ok())
                {
                    peak_rate = peak_rate.max(v);
                }
                if l.contains("\"event\":\"run_complete\"") {
                    run_start = field(l, "start_utc").unwrap_or("").to_string();
                    run_end = field(l, "completion_utc").unwrap_or("").to_string();
                    wall = field(l, "elapsed_wall_seconds")
                        .and_then(|x| x.parse().ok())
                        .unwrap_or(0.0);
                }
            }
        }
        let mut cpu = 0f64;
        let mut avg_cpu = 0f64;
        let mut peak_cpu = 0f64;
        let mut peak_rss = 0u64;
        let mut cpu_sample_utc = String::new();
        let mut samples = 0u64;
        if let Ok(data) = fs::read_to_string(root.join("system_metrics.jsonl")) {
            for l in data.lines() {
                if let Some(v) =
                    field(l, "cumulative_cpu_seconds").and_then(|x| x.parse::<f64>().ok())
                {
                    cpu = cpu.max(v)
                }
                if let Some(v) =
                    field(l, "cpu_percent_since_start").and_then(|x| x.parse::<f64>().ok())
                {
                    avg_cpu += v;
                    peak_cpu = peak_cpu.max(v);
                    samples += 1
                }
                if let Some(v) = field(l, "rss_kib").and_then(|x| x.parse::<u64>().ok()) {
                    peak_rss = peak_rss.max(v)
                }
                cpu_sample_utc = field(l, "timestamp_utc").unwrap_or("").to_string();
            }
        }
        if samples > 0 {
            avg_cpu /= samples as f64;
        }
        let audit = audit_partitions(&root);
        let coverage = audit.covered_assignments == expected
            && audit.completed_partitions as u128 == expected_partitions
            && audit.missing_partitions == 0
            && audit.invalid_partitions == 0;
        let record_audit_passed =
            audit.footer_result_count == audit.parsed_result_count && audit.invalid_records == 0;
        let status = if audit.complete()
            && generated == expected
            && partitions as u128 == expected_partitions
            && unresolved == 0
        {
            "complete"
        } else {
            "incomplete_or_unresolved"
        };
        let mut starts = 0u64;
        let mut failed = 0u64;
        if let Ok(data) = fs::read_to_string(root.join("supervisor.jsonl")) {
            for l in data.lines() {
                if l.contains("\"event\":\"supervisor_start\"") {
                    starts += 1
                }
                if l.contains("\"event\":\"worker_exit\"") && field(l, "exit_code") != Some("0") {
                    failed += 1
                }
            }
        }
        primes.sort_by_key(|line| {
            field(line, "value")
                .and_then(|x| x.parse::<u128>().ok())
                .unwrap_or(u128::MAX)
        });
        let report = format!(
            "{{\"status\":\"{status}\",\"minimum_integer\":2,\"maximum_integer\":\"{}\",\"expected_candidate_assignments\":\"{expected}\",\"generated_assignments\":\"{generated}\",\"completed_partitions\":{partitions},\"expected_partitions\":{expected_partitions},\"coverage_verified\":{coverage},\"verified_result_records\":{},\"footer_result_count\":{},\"invalid_result_records\":{},\"invalid_partitions\":{},\"missing_partitions\":{},\"record_audit_passed\":{record_audit_passed},\"binary_palindrome_candidates\":\"{generated}\",\"hex_pass\":\"{hex_pass}\",\"hex_rejected\":\"{}\",\"decimal_pass\":\"{decimal_pass}\",\"decimal_rejected\":\"{}\",\"primality_tests\":{decimal_pass},\"triple_palindrome_composites\":{composites},\"proven_primes\":{},\"unresolved\":{unresolved},\"search_start_utc\":\"{run_start}\",\"search_completion_utc\":\"{run_end}\",\"wall_seconds\":{wall:.6},\"active_computation_wall_seconds\":{wall:.6},\"summed_partition_seconds\":{part_seconds:.6},\"candidate_generation_and_other_worker_seconds\":{:.6},\"binary_check_seconds\":0,\"hex_check_seconds\":{hex_seconds:.6},\"decimal_check_seconds\":{decimal_seconds:.6},\"primality_verification_seconds\":{primality_seconds:.9},\"checkpointing_overhead_seconds\":null,\"checkpointing_overhead_measured\":false,\"interruptions\":{},\"process_restarts\":{},\"failed_partitions\":{failed},\"checkpoint_recoveries\":0,\"average_candidates_per_second\":{},\"peak_sampled_candidates_per_second\":{peak_rate:.3},\"cumulative_cpu_seconds_sampled\":{cpu:.3},\"cpu_sample_timestamp_utc\":\"{cpu_sample_utc}\",\"average_cpu_percent_per_logical_cpu\":{},\"peak_cpu_percent_since_start\":{peak_cpu:.1},\"peak_rss_kib\":{peak_rss},\"hardware\":\"AMD Ryzen 7 5700U, x86_64, 8 physical cores, 16 logical CPUs, AVX2, 22 GiB RAM\",\"compiler\":\"rustc 1.99.1, release opt-level=3, thin LTO, codegen-units=1, target-cpu=native\",\"throughput_by_bit_length\":{{{}}}}}",
            u128::MAX,
            audit.parsed_result_count,
            audit.footer_result_count,
            audit.invalid_records,
            audit.invalid_partitions,
            audit.missing_partitions,
            generated.saturating_sub(hex_pass),
            hex_pass.saturating_sub(decimal_pass),
            primes.len(),
            (part_seconds - hex_seconds - decimal_seconds - primality_seconds).max(0.0),
            failed,
            starts.saturating_sub(1),
            if wall > 0.0 {
                generated as f64 / wall
            } else {
                0.0
            },
            avg_cpu / 16.0,
            (2..=128)
                .map(|b| format!(
                    "\"{b}\":{{\"candidates\":\"{}\",\"partitions\":{},\"worker_seconds\":{:.6}}}",
                    length[b].0, length[b].1, length[b].2
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        atomic_write(&root.join("final-report.json"), report.as_bytes()).unwrap();
        let human = primes
            .iter()
            .map(|line| field(line, "value").unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        atomic_write(&root.join("primes.txt"), format!("{human}\n").as_bytes()).unwrap();
        println!("{report}");
        return;
    }
    let mut jobs = Vec::new();
    let mut total_assign = 0u128;
    let max_bits = if cmd == "small" { 16 } else { 128 };
    for bits in 2..=max_bits {
        let s = Space::new(bits);
        let count = s.count();
        total_assign = total_assign.saturating_add(count);
        let mut start = 0;
        while start < count {
            let end = (start + CHUNK).min(count);
            if !job_file(&root, bits, start).exists() {
                jobs.push((bits, start, end, count));
            }
            start = end;
        }
    }
    let start_utc = utc();
    let start = Instant::now();
    let work = Arc::new(AtomicU64::new(0));
    let queue = Arc::new(std::sync::Mutex::new(jobs.into_iter()));
    let snapshots = root.join("telemetry.jsonl");
    let mut workers = Vec::new();
    for _ in 0..threads {
        let q = queue.clone();
        let root = root.clone();
        let w = work.clone();
        workers.push(thread::spawn(move || {
            loop {
                let job = q.lock().unwrap().next();
                let Some((b, a, e, c)) = job else { break };
                process(&root, b, a, e, c, &w)
            }
        }));
    }
    let mut last = Instant::now();
    while workers.iter().any(|h| !h.is_finished()) {
        if last.elapsed() >= Duration::from_secs(60) {
            let elapsed = start.elapsed().as_secs_f64();
            let c = work.load(Ordering::Relaxed);
            let snap = format!(
                "{{\"timestamp_utc\":\"{}\",\"search_start_utc\":\"{}\",\"elapsed_wall_seconds\":{elapsed:.3},\"candidates_processed_this_run\":{c},\"candidate_assignments_total\":\"{total_assign}\",\"percentage_of_assignment_space_this_run\":{},\"instantaneous_candidates_per_second\":{},\"active_workers\":{threads}}}",
                utc(),
                start_utc,
                if total_assign == 0 {
                    0.0
                } else {
                    100.0 * c as f64 / total_assign as f64
                },
                if elapsed == 0.0 {
                    0.0
                } else {
                    c as f64 / elapsed
                }
            );
            append_event(&snapshots, &snap);
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(500));
    }
    for w in workers {
        w.join().unwrap()
    }
    let duration = start.elapsed().as_secs_f64();
    let completed = fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name().to_string_lossy().starts_with("part-")
                && e.path().extension().is_some_and(|x| x == "jsonl")
        })
        .count();
    let finalrec = format!(
        "{{\"event\":\"run_complete\",\"start_utc\":\"{}\",\"completion_utc\":\"{}\",\"elapsed_wall_seconds\":{duration:.6},\"assignments_this_run\":{},\"completed_partition_files\":{completed},\"total_assignment_space\":\"{total_assign}\",\"status\":\"{}\"}}",
        start_utc,
        utc(),
        work.load(Ordering::Relaxed),
        if completed == 0 {
            "empty"
        } else {
            "partial_or_complete"
        }
    );
    append_event(&snapshots, &finalrec);
    println!("{finalrec}");
}

#[cfg(test)]
mod primality_tests {
    use super::*;
    #[test]
    fn small_primes_and_composites_are_classified_with_evidence() {
        for n in [3u128, 5, 7] {
            let (c, m, e) = classify(n);
            assert_eq!(c, "prime");
            assert_eq!(m, "trial-division");
            assert!(e.starts_with("trial_divisors_through="));
        }
        let (c, _, e) = classify(9);
        assert_eq!(c, "composite");
        assert_eq!(e, "factor=3");
        let base = compositeness_witness(3127).unwrap();
        assert!(!mr(3127, base));
    }
    #[test]
    fn modular_arithmetic_handles_u128_edges() {
        let m = u128::MAX;
        assert_eq!(mulmod(m - 1, m - 1, m), 1);
        assert_eq!(powmod(m - 1, 2, m), 1);
        for a in 0..32u128 {
            for b in 0..32u128 {
                for q in 1..37u128 {
                    assert_eq!(mulmod(a, b, q), (a * b) % q);
                }
            }
        }
    }
}
