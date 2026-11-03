use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
struct Dsu {
    p: Vec<usize>,
    rank: Vec<u8>,
}
impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            p: (0..n).collect(),
            rank: vec![0; n],
        }
    }
    fn find(&mut self, x: usize) -> usize {
        if self.p[x] != x {
            self.p[x] = self.find(self.p[x]);
        }
        self.p[x]
    }
    fn union(&mut self, a: usize, b: usize) {
        let (mut x, mut y) = (self.find(a), self.find(b));
        if x == y {
            return;
        }
        if self.rank[x] < self.rank[y] {
            std::mem::swap(&mut x, &mut y)
        }
        self.p[y] = x;
        if self.rank[x] == self.rank[y] {
            self.rank[x] += 1
        }
    }
}
#[derive(Clone, Debug)]
pub struct Space {
    pub bits: u32,
    base: u128,
    masks: Vec<u128>,
    pub free_bits: u32,
}

#[derive(Clone, Debug)]
pub struct BigSpace {
    pub bits: u32,
    base: BigUint,
    masks: Vec<BigUint>,
    pub free_bits: u32,
}
impl BigSpace {
    pub fn new(bits: u32) -> Self {
        assert!((2..=256).contains(&bits));
        let l = bits as usize;
        let h = (l + 3) / 4;
        let r = if l % 4 == 0 { 4 } else { l % 4 };
        let zero = l;
        let mut d = Dsu::new(l + 1);
        for i in 0..l {
            d.union(i, l - 1 - i)
        }
        for j in 0..h / 2 {
            let lo = j;
            let hi = h - 1 - j;
            if lo == 0 {
                for k in 0..r {
                    d.union(k, 4 * hi + k)
                }
                for k in r..4 {
                    d.union(k, zero)
                }
            } else {
                for k in 0..4 {
                    d.union(4 * lo + k, 4 * hi + k)
                }
            }
        }
        let z = d.find(zero);
        let top = d.find(l - 1);
        let mut roots = Vec::<usize>::new();
        let mut posroot = Vec::with_capacity(l);
        for i in 0..l {
            posroot.push(d.find(i));
        }
        for &root in &posroot {
            if root != z && root != top && !roots.contains(&root) {
                roots.push(root)
            }
        }
        let mut base = BigUint::zero();
        base.set_bit((l - 1) as u64, true);
        let mut masks = vec![BigUint::zero(); roots.len()];
        for i in 0..l {
            let root = posroot[i];
            if root == top {
                base.set_bit(i as u64, true)
            } else if root != z {
                let id = roots.iter().position(|&x| x == root).unwrap();
                masks[id].set_bit(i as u64, true);
            }
        }
        Self {
            bits,
            base,
            masks,
            free_bits: roots.len() as u32,
        }
    }
    pub fn count(&self) -> BigUint {
        BigUint::one() << self.free_bits as usize
    }
    pub fn candidate(&self, a: &BigUint) -> BigUint {
        assert!(a < &self.count());
        let mut n = self.base.clone();
        for (i, m) in self.masks.iter().enumerate() {
            if a.bit(i as u64) {
                n |= m;
            }
        }
        n
    }
    pub fn gray_start(&self, a: &BigUint) -> BigUint {
        let g = a ^ (a >> 1usize);
        let mut n = self.base.clone();
        for (i, m) in self.masks.iter().enumerate() {
            if g.bit(i as u64) {
                n |= m;
            }
        }
        n
    }
    pub fn gray_flip_mask(&self, next: &BigUint) -> BigUint {
        let bit = next.trailing_zeros().expect("zero has no Gray transition") as usize;
        self.masks[bit].clone()
    }
    /// Invert the constrained-bit mapping and Gray-code order for a candidate.
    /// Returns its unique assignment index, or `None` when it is not in this space.
    pub fn assignment_index(&self, n: &BigUint) -> Option<BigUint> {
        if n.bits() != self.bits as u64 {
            return None;
        }
        let mut rebuilt = self.base.clone();
        let mut gray = BigUint::zero();
        for (i, mask) in self.masks.iter().enumerate() {
            let selected = n & mask;
            if selected.is_zero() {
                continue;
            }
            if &selected != mask {
                return None;
            }
            rebuilt |= mask;
            gray.set_bit(i as u64, true);
        }
        if &rebuilt != n {
            return None;
        }
        let mut index = BigUint::zero();
        let mut prefix = gray;
        while !prefix.is_zero() {
            index ^= &prefix;
            prefix >>= 1usize;
        }
        Some(index)
    }
}
impl Space {
    pub fn new(bits: u32) -> Self {
        assert!((2..=128).contains(&bits));
        let l = bits as usize;
        let h = (l + 3) / 4;
        let lead = l % 4;
        let lead = if lead == 0 { 4 } else { lead };
        let zero = l;
        let mut d = Dsu::new(l + 1);
        for i in 0..l {
            d.union(i, l - 1 - i)
        }
        for j in 0..h / 2 {
            let lo = j;
            let hi = h - 1 - j;
            if lo == 0 {
                for k in 0..lead {
                    d.union(k, 4 * hi + k)
                }
                for k in lead..4 {
                    d.union(k, zero)
                }
            } else {
                for k in 0..4 {
                    d.union(4 * lo + k, 4 * hi + k)
                }
            }
        }
        let z = d.find(zero);
        let top = d.find(l - 1);
        let mut roots = Vec::<usize>::new();
        let mut posroot = Vec::with_capacity(l);
        for i in 0..l {
            posroot.push(d.find(i));
        }
        for &r in &posroot {
            if r != z && r != top && !roots.contains(&r) {
                roots.push(r)
            }
        }
        let mut base = 1u128 << (l - 1);
        let mut masks = vec![0u128; roots.len()];
        for i in 0..l {
            let r = posroot[i];
            if r == top {
                base |= 1u128 << i
            } else if r != z {
                let id = roots.iter().position(|&x| x == r).unwrap();
                masks[id] |= 1u128 << i;
            }
        }
        Self {
            bits,
            base,
            masks,
            free_bits: roots.len() as u32,
        }
    }
    pub fn count(&self) -> u128 {
        1u128
            .checked_shl(self.free_bits)
            .expect("candidate space exceeds u128")
    }
    pub fn candidate(&self, a: u128) -> u128 {
        assert!(a < self.count());
        let mut n = self.base;
        let mut x = a;
        while x != 0 {
            let b = x.trailing_zeros() as usize;
            n |= self.masks[b];
            x &= x - 1;
        }
        n
    }
    pub fn gray_start(&self, a: u128) -> u128 {
        let g = a ^ (a >> 1);
        let mut n = self.base;
        let mut x = g;
        while x != 0 {
            let b = x.trailing_zeros() as usize;
            n |= self.masks[b];
            x &= x - 1;
        }
        n
    }
    pub fn gray_flip_mask(&self, next_index: u128) -> u128 {
        self.masks[(next_index.trailing_zeros()) as usize]
    }
}
pub fn binary_pal(n: u128) -> bool {
    if n == 0 {
        return false;
    }
    let l = 128 - n.leading_zeros();
    for i in 0..l / 2 {
        if ((n >> i) & 1) != ((n >> (l - 1 - i)) & 1) {
            return false;
        }
    }
    true
}
pub fn hex_pal(mut n: u128) -> bool {
    if n == 0 {
        return false;
    }
    let mut ds = [0u8; 32];
    let mut len = 0;
    while n > 0 {
        ds[len] = (n & 15) as u8;
        len += 1;
        n >>= 4
    }
    for i in 0..len / 2 {
        if ds[i] != ds[len - 1 - i] {
            return false;
        }
    }
    true
}
pub fn decimal_pal(mut n: u128) -> bool {
    if n == 0 {
        return false;
    }
    let orig = n;
    let mut rev = 0u128;
    while n > 0 {
        let q = n / 10;
        let d = n % 10;
        let Some(x) = rev.checked_mul(10).and_then(|v| v.checked_add(d)) else {
            return false;
        };
        rev = x;
        n = q
    }
    rev == orig
}
pub fn reference_pal(mut n: u128, base: u32) -> bool {
    if n == 0 {
        return false;
    }
    let mut ds = Vec::new();
    while n > 0 {
        ds.push((n % base as u128) as u32);
        n /= base as u128;
    }
    ds.iter().eq(ds.iter().rev())
}
pub fn binary_pal_big(n: &BigUint) -> bool {
    let l = n.bits();
    if l == 0 {
        return false;
    }
    (0..l / 2).all(|i| n.bit(i) == n.bit(l - 1 - i))
}
pub fn hex_pal_big(n: &BigUint) -> bool {
    let s = n.to_str_radix(16);
    s.as_bytes().iter().eq(s.as_bytes().iter().rev())
}
pub fn decimal_pal_big(n: &BigUint) -> bool {
    let s = n.to_str_radix(10);
    s.as_bytes().iter().eq(s.as_bytes().iter().rev())
}
pub fn utc() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let days = (d.as_secs() / 86_400) as i64;
    let sod = d.as_secs() % 86_400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:09}Z",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60,
        d.subsec_nanos()
    )
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_palindromes() {
        for n in 1..50000u128 {
            assert_eq!(binary_pal(n), reference_pal(n, 2), "bin {n}");
            assert_eq!(hex_pal(n), reference_pal(n, 16), "hex {n}");
            assert_eq!(decimal_pal(n), reference_pal(n, 10), "dec {n}");
        }
    }
    #[test]
    fn candidate_generation_is_exact_small() {
        for bits in 2..=20 {
            let s = Space::new(bits);
            let mut got = Vec::new();
            for a in 0..s.count() {
                let n = s.candidate(a);
                assert!(binary_pal(n));
                assert!(hex_pal(n));
                got.push(n);
            }
            got.sort_unstable();
            got.dedup();
            let lo = 1u128 << (bits - 1);
            let hi = 1u128 << bits;
            let want: Vec<_> = (lo..hi).filter(|&n| binary_pal(n) && hex_pal(n)).collect();
            assert_eq!(got, want, "bits={bits}");
        }
    }
    #[test]
    fn no_duplicate_assignments() {
        for bits in 2..=64 {
            let s = Space::new(bits);
            let count = s.count();
            if count > 2_000_000 {
                continue;
            }
            let mut seen = std::collections::HashSet::new();
            for a in 0..count {
                assert!(seen.insert(s.candidate(a)), "bits={bits} assignment={a}");
            }
        }
    }
    #[test]
    fn boundaries() {
        for n in [1, 2, 3, 15, 16, 17, u128::MAX - 1, u128::MAX] {
            for (b, f) in [
                (2, binary_pal as fn(u128) -> bool),
                (16, hex_pal),
                (10, decimal_pal),
            ] {
                assert_eq!(f(n), reference_pal(n, b));
            }
        }
        assert_eq!(u128::MAX.checked_add(1), None);
    }
    #[test]
    fn small_known_tri_palindromes() {
        for n in [3u128, 5, 7] {
            assert!(binary_pal(n) && hex_pal(n) && decimal_pal(n));
        }
    }
    #[test]
    fn powers_of_two_and_sixteen_reference_edges() {
        for e in 0..128 {
            let n = 1u128 << e;
            assert_eq!(binary_pal(n), reference_pal(n, 2));
            assert_eq!(hex_pal(n), reference_pal(n, 16));
            assert_eq!(decimal_pal(n), reference_pal(n, 10));
        }
        for e in 0..32 {
            let n = 1u128 << (4 * e);
            assert_eq!(binary_pal(n), reference_pal(n, 2));
            assert_eq!(hex_pal(n), reference_pal(n, 16));
            assert_eq!(decimal_pal(n), reference_pal(n, 10));
        }
    }
}
