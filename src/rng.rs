//! Deterministic randomness: xoshiro256** seeded from (seed, keys...) with distributions built
//! on `libm`, so results are identical across platforms and crate versions.

#[derive(Clone)]
pub struct Rng {
    s: [u64; 4],
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stable 64-bit hash of a string (FNV-1a), for routing and seeding by name.
pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    let mut x = h;
    splitmix(&mut x)
}

impl Rng {
    pub fn new(seed: u64, keys: &[u64]) -> Rng {
        let mut x = seed ^ 0xD1B5_4A32_D192_ED03;
        for &k in keys {
            x = splitmix(&mut x) ^ k.wrapping_mul(0xA24B_AED4_963E_E407);
        }
        let mut s = [0u64; 4];
        for v in s.iter_mut() {
            *v = splitmix(&mut x);
        }
        Rng { s }
    }

    #[inline]
    pub fn u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn f64(&mut self) -> f64 {
        (self.u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform integer in [lo, hi] (inclusive).
    #[inline]
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        lo + (((self.u64() as u128) * (span as u128)) >> 64) as i64
    }

    /// Uniform index in [0, n).
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (((self.u64() as u128) * (n as u128)) >> 64) as usize
    }

    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn normal(&mut self) -> f64 {
        // Box-Muller (one value per call; simple and deterministic)
        let u1 = 1.0 - self.f64();
        let u2 = self.f64();
        libm::sqrt(-2.0 * libm::log(u1)) * libm::cos(2.0 * std::f64::consts::PI * u2)
    }

    pub fn gauss(&mut self, mean: f64, sd: f64) -> f64 {
        mean + sd * self.normal()
    }

    pub fn lognormal(&mut self, mu: f64, sigma: f64) -> f64 {
        libm::exp(mu + sigma * self.normal())
    }

    /// Exponential with the given mean.
    pub fn exp(&mut self, mean: f64) -> f64 {
        -mean * libm::log(1.0 - self.f64())
    }

    pub fn poisson(&mut self, lambda: f64) -> u64 {
        if lambda <= 0.0 {
            return 0;
        }
        if lambda < 30.0 {
            let l = libm::exp(-lambda);
            let mut k = 0u64;
            let mut p = 1.0;
            loop {
                p *= self.f64();
                if p <= l {
                    return k;
                }
                k += 1;
            }
        }
        let v = self.gauss(lambda, libm::sqrt(lambda)).round();
        if v < 0.0 { 0 } else { v as u64 }
    }

    pub fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.below(i + 1);
            xs.swap(i, j);
        }
    }

    /// Random uuid v4 as u128.
    pub fn uuid(&mut self) -> u128 {
        let v = ((self.u64() as u128) << 64) | self.u64() as u128;
        (v & !(0xF000u128 << 64) | (0x4000u128 << 64)) & !(0xC000u128 << 48) | (0x8000u128 << 48)
    }

    pub fn alnum(&mut self, n: usize) -> String {
        const A: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        (0..n).map(|_| A[self.below(A.len())] as char).collect()
    }

    pub fn digits(&mut self, n: usize) -> String {
        (0..n).map(|_| (b'0' + self.below(10) as u8) as char).collect()
    }

    pub fn hex(&mut self, n: usize) -> String {
        const H: &[u8] = b"0123456789abcdef";
        (0..n).map(|_| H[self.below(16)] as char).collect()
    }
}

/// Weighted sampling by binary search over cumulative weights.
#[derive(Clone, Debug)]
pub struct Weighted {
    cum: Vec<f64>,
}

impl Weighted {
    pub fn new(weights: impl IntoIterator<Item = f64>) -> Weighted {
        let mut cum = Vec::new();
        let mut acc = 0.0;
        for w in weights {
            debug_assert!(w >= 0.0);
            acc += w;
            cum.push(acc);
        }
        assert!(acc > 0.0, "Weighted: all weights are zero");
        Weighted { cum }
    }

    pub fn total(&self) -> f64 {
        *self.cum.last().unwrap()
    }

    pub fn sample(&self, rng: &mut Rng) -> usize {
        let x = rng.f64() * self.total();
        self.cum.partition_point(|&c| c <= x).min(self.cum.len() - 1)
    }
}

pub fn format_uuid(u: u128) -> String {
    let h = format!("{u:032x}");
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}
