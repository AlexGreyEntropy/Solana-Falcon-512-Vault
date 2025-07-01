// Number Theoretic Transform (NTT) for Falcon-512
// Designed for no_std environments with minimal compute unit usage

// NTT parameters for Falcon-512
pub const Q: u32 = 12289;  // Prime modulus
pub const N: usize = 512;  // Ring dimension
pub const ROOT_OF_UNITY: u32 = 1479;  // Primitive 1024th root of unity mod Q

// modular inverse of N for inverse NTT
const INV_N: u32 = 12265; // N^(-1) mod Q

// compute twiddle factors on-demand
fn compute_twiddles() -> [u32; N] {
    let mut twiddles = [0u32; N];
    for i in 0..N {
        twiddles[i] = mod_pow_runtime(ROOT_OF_UNITY, i as u32);
    }
    twiddles
}

// compute inverse twiddle factors on-demand
fn compute_inv_twiddles() -> [u32; N] {
    let mut inv_twiddles = [0u32; N];
    for i in 0..N {
        // ω^(-i) = ω^(1024-i) for 1024th root of unity
        inv_twiddles[i] = if i == 0 { 1 } else { mod_pow_runtime(ROOT_OF_UNITY, 1024 - i as u32) };
    }
    inv_twiddles
}

// runtime modular exponentiation using binary method
fn mod_pow_runtime(mut base: u32, mut exp: u32) -> u32 {
    let mut result = 1;
    base %= Q;
    while exp > 0 {
        if exp & 1 == 1 {
            result = fast_mod_q((result as u64 * base as u64) as u32);
        }
        exp >>= 1;
        base = fast_mod_q((base as u64 * base as u64) as u32);
    }
    result
}

// modular multiplication using fast reduction
fn mod_mul(a: u32, b: u32) -> u32 {
    let product = (a as u64) * (b as u64);
    fast_mod_q(product as u32)
}



//fast modular reduction for Q = 12289
// uses the fact that 2^13 ≡ 13 (mod 12289)
#[inline]
pub fn fast_mod_q(x: u32) -> u32 {
    if x < Q {
        x
    } else {
        let high = x >> 13;
        let low = x & 8191; // x & (2^13 - 1)
        let reduced = low + 13 * high;
        if reduced >= Q { reduced - Q } else { reduced }
    }
}



// bit-reverse a value for NTT input/output ordering
#[inline]
fn bit_reverse(mut x: usize, bits: u32) -> usize {
    let mut result = 0;
    for _ in 0..bits {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    result
}

// forward NTT transformation
// it transforms coefficients from time domain to frequency domain
pub fn ntt_forward(coeffs: &mut [u32; N]) {
    let twiddle_factors = compute_twiddles();
    
    // bit-reverse input for decimation-in-frequency NTT
    for i in 0..N {
        let j = bit_reverse(i, 9); // log2(512) = 9
        if i < j {
            coeffs.swap(i, j);
        }
    }
    
    // NTT with decimation-in-frequency
    let mut len = 2;
    while len <= N {
        let step = N / len;
        for start in (0..N).step_by(len) {
            let mut j = 0;
            for i in start..start + len / 2 {
                let u = coeffs[i];
                let v = mod_mul(coeffs[i + len / 2], twiddle_factors[step * j]);
                
                coeffs[i] = fast_mod_q(u + v);
                coeffs[i + len / 2] = fast_mod_q(u + Q - v);
                
                j += 1;
            }
        }
        len <<= 1;
    }
}

// inverse NTT transformation
// this transforms coefficients from frequency domain back to time domain
pub fn ntt_inverse(coeffs: &mut [u32; N]) {
    let inv_twiddle_factors = compute_inv_twiddles();
    
    // inverse NTT
    let mut len = N;
    while len >= 2 {
        let step = N / len;
        for start in (0..N).step_by(len) {
            let mut j = 0;
            for i in start..start + len / 2 {
                let u = coeffs[i];
                let v = coeffs[i + len / 2];
                
                coeffs[i] = fast_mod_q(u + v);
                coeffs[i + len / 2] = mod_mul(fast_mod_q(u + Q - v), inv_twiddle_factors[step * j]);
                
                j += 1;
            }
        }
        len >>= 1;
    }
    
    //scale by 1/N
    for coeff in coeffs.iter_mut() {
        *coeff = mod_mul(*coeff, INV_N);
    }
    
    // bit-reverse output
    for i in 0..N {
        let j = bit_reverse(i, 9);
        if i < j {
            coeffs.swap(i, j);
        }
    }
}

//pointwise multiplication in NTT domain
// more efficient than polynomial multiplication in time domain
#[inline]
pub fn ntt_pointwise_mul(a: &[u32; N], b: &[u32; N], result: &mut [u32; N]) {
    for i in 0..N {
        result[i] = mod_mul(a[i], b[i]);
    }
}

// subtract two polynomials in NTT domain
#[inline]
pub fn ntt_pointwise_sub(a: &[u32; N], b: &[u32; N], result: &mut [u32; N]) {
    for i in 0..N {
        result[i] = fast_mod_q(a[i] + Q - b[i]);
    }
}

// convert signed coefficients to unsigned for NTT
pub fn to_ntt_form(signed_coeffs: &[i16; N]) -> [u32; N] {
    let mut unsigned_coeffs = [0u32; N];
    for i in 0..N {
        //convert from signed to unsigned representation in Z_q
        let val = signed_coeffs[i] as i32;
        unsigned_coeffs[i] = if val >= 0 {
            val as u32
        } else {
            (val + Q as i32) as u32
        };
    }
    unsigned_coeffs
}

// convert unsigned coefficients back to signed form
pub fn from_ntt_form(unsigned_coeffs: &[u32; N]) -> [i16; N] {
    let mut signed_coeffs = [0i16; N];
    for i in 0..N {
        let val = unsigned_coeffs[i];
        signed_coeffs[i] = if val > Q / 2 {
            (val as i32 - Q as i32) as i16
        } else {
            val as i16
        };
    }
    signed_coeffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntt_roundtrip() {
        let mut coeffs = [0u32; N];
        // set up a simple test pattern
        for i in 0..10 {
            coeffs[i] = i as u32 + 1;
        }
        
        let original = coeffs;
        
        // forward NTT
        ntt_forward(&mut coeffs);
        
        // Inverse NTT
        ntt_inverse(&mut coeffs);
        
        // this should recover original coefficients
        for i in 0..N {
            assert_eq!(coeffs[i], original[i]);
        }
    }

    #[test]
    fn test_modular_arithmetic() {
        assert_eq!(fast_mod_q(Q), 0);
        assert_eq!(fast_mod_q(Q - 1), Q - 1);
        assert_eq!(fast_mod_q(Q + 1), 1);
        assert_eq!(fast_mod_q(2 * Q), 0);
    }

    #[test]
    fn test_conversion() {
        let signed = [-1i16, 0, 1, -6144, 6144];
        let mut test_coeffs = [0i16; N];
        test_coeffs[..5].copy_from_slice(&signed);
        
        let unsigned = to_ntt_form(&test_coeffs);
        let recovered = from_ntt_form(&unsigned);
        
        assert_eq!(test_coeffs[..5], recovered[..5]);
    }
} 