use multi_pal_prime::{Space, decimal_pal, hex_pal};
use std::time::Instant;
fn main() {
    for bits in [63u32, 95, 123, 127, 128] {
        let s = Space::new(bits);
        let count = 1_000_000u128.min(s.count());
        let t = Instant::now();
        let mut n = s.gray_start(0);
        let mut hex = 0u64;
        let mut dec = 0u64;
        for i in 0..count {
            if hex_pal(n) {
                hex += 1;
                if decimal_pal(n) {
                    dec += 1
                }
            }
            if i + 1 < count {
                n ^= s.gray_flip_mask(i + 1)
            }
        }
        let sec = t.elapsed().as_secs_f64();
        println!(
            "bits={bits} candidates={count} seconds={sec:.6} candidates_per_second={:.0} hex_pass={hex} decimal_pass={dec}",
            count as f64 / sec
        );
    }
    bench_big();
}

use multi_pal_prime::{BigSpace, binary_pal_big, decimal_pal_big, hex_pal_big};
use num_bigint::BigUint;
fn bench_big() {
    let bits = 256u32;
    let s = BigSpace::new(bits);
    let count = 100_000u64;
    let mut index = BigUint::from(0u8);
    let mut n = s.gray_start(&index);
    let t = Instant::now();
    let mut decimal = 0u64;
    for step in 0..count {
        if binary_pal_big(&n) && hex_pal_big(&n) && decimal_pal_big(&n) {
            decimal += 1
        }
        if step + 1 < count {
            index += 1u8;
            n ^= s.gray_flip_mask(&index)
        }
    }
    let sec = t.elapsed().as_secs_f64();
    println!(
        "big_bits={bits} candidates={count} seconds={sec:.6} candidates_per_second={:.0} decimal_pass={decimal}",
        count as f64 / sec
    );
}
