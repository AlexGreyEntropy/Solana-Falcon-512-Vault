// Falcon-512 verification for no_std environments
// Falcon specification and optimized for Solana

use pinocchio::program_error::ProgramError;
use core::ops::{Add, Sub, Mul, Neg};

// Falcon-512 public key and signature sizes
pub const FALCON_512_PUBLIC_KEY_SIZE: usize = 897;
pub const FALCON_512_SIGNATURE_SIZE: usize = 666;
pub const FALCON_512_N: usize = 512;
pub const FALCON_512_Q: u16 = 12289;
pub const FALCON_512_LOGN: usize = 9;

// fixed-point arithmetic for no_std compatibility
const FIXED_POINT_SCALE: i64 = 1 << 32;

// convert floating point to fixed point for bounds checking
const fn float_to_fixed(f: f64) -> i64 {
    (f * FIXED_POINT_SCALE as f64) as i64
}

// Falcon-512 signature bound (converted to fixed point)
const FALCON_512_SIG_BOUND_FIXED: i64 = float_to_fixed(34034726.0);

// field element in Z_q
#[derive(Clone, Copy, Debug, PartialEq)]
struct FieldElement(u16);

impl FieldElement {
    fn new(value: u16) -> Self {
        Self(value % FALCON_512_Q)
    }
    
    fn value(self) -> u16 {
        self.0
    }
    
    fn balanced_value(self) -> i16 {
        let v = self.0 as i32;
        if v > (FALCON_512_Q as i32) / 2 {
            (v - FALCON_512_Q as i32) as i16
        } else {
            v as i16
        }
    }
}

impl Add for FieldElement {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.0 + other.0)
    }
}

impl Sub for FieldElement {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.0 + FALCON_512_Q - other.0)
    }
}

impl Mul for FieldElement {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self::new(((self.0 as u32 * other.0 as u32) % FALCON_512_Q as u32) as u16)
    }
}

impl Neg for FieldElement {
    type Output = Self;
    fn neg(self) -> Self {
        if self.0 == 0 {
            self
        } else {
            Self(FALCON_512_Q - self.0)
        }
    }
}

// polynomial in the ring Z_q[X]/(X^n + 1)
#[derive(Clone)]
struct Polynomial {
    coeffs: [FieldElement; FALCON_512_N],
}

impl Polynomial {
    fn zero() -> Self {
        Self {
            coeffs: [FieldElement(0); FALCON_512_N],
        }
    }
    
    fn from_coeffs(coeffs: [FieldElement; FALCON_512_N]) -> Self {
        Self { coeffs }
    }
    
    fn from_signed_coeffs(signed_coeffs: &[i16; FALCON_512_N]) -> Self {
        let mut coeffs = [FieldElement(0); FALCON_512_N];
        for i in 0..FALCON_512_N {
            let val = ((signed_coeffs[i] as i32 % FALCON_512_Q as i32 + FALCON_512_Q as i32) % FALCON_512_Q as i32) as u16;
            coeffs[i] = FieldElement(val);
        }
        Self::from_coeffs(coeffs)
    }
    
    // NTT transformation (forward)
    fn ntt(&self) -> Self {
        let mut coeffs_u32 = [0u32; FALCON_512_N];
        
        // convert to u32 representation
        for i in 0..FALCON_512_N {
            coeffs_u32[i] = self.coeffs[i].value() as u32;
        }
        
        // perform NTT
        super::ntt::ntt_forward(&mut coeffs_u32);
        
        // convert back to FieldElement
        let mut result_coeffs = [FieldElement(0); FALCON_512_N];
        for i in 0..FALCON_512_N {
            result_coeffs[i] = FieldElement::new(coeffs_u32[i] as u16);
        }
        
        Self::from_coeffs(result_coeffs)
    }
    
    // Inverse NTT transformation
    fn intt(&self) -> Self {
        let mut coeffs_u32 = [0u32; FALCON_512_N];
        
        // Convert to u32 representation
        for i in 0..FALCON_512_N {
            coeffs_u32[i] = self.coeffs[i].value() as u32;
        }
        
        // perform inverse NTT
        super::ntt::ntt_inverse(&mut coeffs_u32);
        
        // convert back to FieldElement
        let mut result_coeffs = [FieldElement(0); FALCON_512_N];
        for i in 0..FALCON_512_N {
            result_coeffs[i] = FieldElement::new(coeffs_u32[i] as u16);
        }
        
        Self::from_coeffs(result_coeffs)
    }
    
    // pointwise multiplication in NTT domain
    fn pointwise_mul(&self, other: &Self) -> Self {
        let mut a_coeffs = [0u32; FALCON_512_N];
        let mut b_coeffs = [0u32; FALCON_512_N];
        let mut result_coeffs = [0u32; FALCON_512_N];
        
        // convert to u32 representation
        for i in 0..FALCON_512_N {
            a_coeffs[i] = self.coeffs[i].value() as u32;
            b_coeffs[i] = other.coeffs[i].value() as u32;
        }
        
        // perform pointwise multiplication
        super::ntt::ntt_pointwise_mul(&a_coeffs, &b_coeffs, &mut result_coeffs);
        
        // convert back to FieldElement
        let mut result_field_coeffs = [FieldElement(0); FALCON_512_N];
        for i in 0..FALCON_512_N {
            result_field_coeffs[i] = FieldElement::new(result_coeffs[i] as u16);
        }
        
        Self::from_coeffs(result_field_coeffs)
    }
}

impl Add for Polynomial {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let mut result = Self::zero();
        for i in 0..FALCON_512_N {
            result.coeffs[i] = self.coeffs[i] + other.coeffs[i];
        }
        result
    }
}

impl Sub for Polynomial {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        let mut result = Self::zero();
        for i in 0..FALCON_512_N {
            result.coeffs[i] = self.coeffs[i] - other.coeffs[i];
        }
        result
    }
}

// hash message to a point in the lattice
// implementing algorithm 3 from Falcon specification
fn hash_to_point(message: &[u8], nonce: &[u8; 40]) -> Polynomial {
    const K: u32 = (1u32 << 16) / FALCON_512_Q as u32;
    
    let mut hasher = super::keccak::Shake256::new();
    hasher.update(nonce);
    hasher.update(message);
    let mut reader = hasher.finalize_xof();
    
    let mut coeffs = [FieldElement(0); FALCON_512_N];
    let mut i = 0;
    
    while i < FALCON_512_N {
        let mut randomness = [0u8; 2];
        reader.read(&mut randomness);
        
        let t = ((randomness[0] as u32) << 8) | (randomness[1] as u32);
        if t < K * FALCON_512_Q as u32 {
            coeffs[i] = FieldElement::new((t % FALCON_512_Q as u32) as u16);
            i += 1;
        }
    }
    
    Polynomial::from_coeffs(coeffs)
}

//decompress Falcon signature from compressed format
// implementation of Algorithm 18 from Falcon specifications
fn decompress_signature(compressed: &[u8]) -> Result<[i16; FALCON_512_N], ProgramError> {
    let mut result = [0i16; FALCON_512_N];
    let mut bit_pos = 0;
    
    for i in 0..FALCON_512_N {
        // read sign bit
        if bit_pos / 8 >= compressed.len() {
            return Err(ProgramError::InvalidAccountData);
        }
        
        let byte_idx = bit_pos / 8;
        let bit_idx = bit_pos % 8;
        let sign = if (compressed[byte_idx] >> bit_idx) & 1 == 1 { -1 } else { 1 };
        bit_pos += 1;
        
        // read value bits (variable length encoding)
        let mut value = 0i16;
        let mut shift = 0;
        
        // read 7 bits at a time until we hit a continuation bit
        loop {
            if bit_pos / 8 >= compressed.len() {
                return Err(ProgramError::InvalidAccountData);
            }
            
            let mut byte_value = 0;
            for _ in 0..7 {
                if bit_pos / 8 >= compressed.len() {
                    return Err(ProgramError::InvalidAccountData);
                }
                let byte_idx = bit_pos / 8;
                let bit_idx = bit_pos % 8;
                byte_value |= ((compressed[byte_idx] >> bit_idx) & 1) << (bit_pos % 7);
                bit_pos += 1;
            }
            
            value |= (byte_value as i16) << shift;
            shift += 7;
            
            // check continuation bit
            if bit_pos / 8 >= compressed.len() {
                return Err(ProgramError::InvalidAccountData);
            }
            let byte_idx = bit_pos / 8;
            let bit_idx = bit_pos % 8;
            let continuation = (compressed[byte_idx] >> bit_idx) & 1;
            bit_pos += 1;
            
            if continuation == 0 {
                break;
            }
            
            if shift > 14 { // this prevents overflow
                return Err(ProgramError::InvalidAccountData);
            }
        }
        
        result[i] = sign * value;
        
        //check for coefficient bounds
        if result[i].abs() > 2048 {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    Ok(result)
}

// parse public key from bytes
fn parse_public_key(pk_bytes: &[u8; FALCON_512_PUBLIC_KEY_SIZE]) -> Result<Polynomial, ProgramError> {
    //header
    let header = pk_bytes[0];
    if header != FALCON_512_LOGN as u8 {
        return Err(ProgramError::InvalidAccountData);
    }
    
    //parse polynomial coefficients (14 bits each, little-endian packed)
    let mut coeffs = [FieldElement(0); FALCON_512_N];
    let data = &pk_bytes[1..]; // skips header
    
    for i in 0..FALCON_512_N {
        let bit_offset = i * 14;
        let byte_offset = bit_offset / 8;
        let bit_pos = bit_offset % 8;
        
        if byte_offset + 2 >= data.len() {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // read the 14 bits spanning potentially 3 bytes
        let mut coeff = 0u16;
        for j in 0..14 {
            let curr_bit_pos = bit_pos + j;
            let curr_byte_offset = byte_offset + curr_bit_pos / 8;
            let curr_bit_idx = curr_bit_pos % 8;
            
            if curr_byte_offset < data.len() {
                let bit = (data[curr_byte_offset] >> curr_bit_idx) & 1;
                coeff |= (bit as u16) << j;
            }
        }
        
        coeffs[i] = FieldElement::new(coeff);
    }
    
    Ok(Polynomial::from_coeffs(coeffs))
}

//parse signature from bytes
fn parse_signature(sig_bytes: &[u8; FALCON_512_SIGNATURE_SIZE]) -> Result<([u8; 40], &[u8]), ProgramError> {
    // chek header
    let header = sig_bytes[0];
    let encoding_type = (header >> 5) & 7;
    let fixed_bit = (header >> 4) & 1;
    let logn = header & 15;
    
    if encoding_type != 2 || fixed_bit != 1 || logn != FALCON_512_LOGN as u8 {
        return Err(ProgramError::InvalidAccountData);
    }
    
    //extract nonce and compressed signature
    if sig_bytes.len() < 41 {
        return Err(ProgramError::InvalidAccountData);
    }
    
    let mut nonce = [0u8; 40];
    nonce.copy_from_slice(&sig_bytes[1..41]);
    let compressed_sig = &sig_bytes[41..];
    
    Ok((nonce, compressed_sig))
}

// this is main Falcon-512 verification function
// verification algorithm from the Falcon specification
pub fn verify_falcon_signature(
    public_key_bytes: &[u8; FALCON_512_PUBLIC_KEY_SIZE],
    signature_bytes: &[u8; FALCON_512_SIGNATURE_SIZE],
    message: &[u8],
) -> Result<(), ProgramError> {
    // parse public key
    let h = parse_public_key(public_key_bytes)?;
    
    //parse signature
    let (nonce, compressed_sig) = parse_signature(signature_bytes)?;
    
    // decompress signature to get s2
    let s2_coeffs = decompress_signature(compressed_sig)?;
    
    // convert s2 to polynomial
    let s2 = Polynomial::from_signed_coeffs(&s2_coeffs);
    
    // hash message to point
    let c = hash_to_point(message, &nonce);
    
    // compute s1 = c - s2 * h (in NTT domain, for efficiency)
    let c_ntt = c.ntt();
    let s2_ntt = s2.ntt();
    let h_ntt = h.ntt();
    
    let s2h_ntt = s2_ntt.pointwise_mul(&h_ntt);
    let s1_ntt = c_ntt - s2h_ntt;
    let s1 = s1_ntt.intt();
    
    //extract signed coefficients for norm check
    let mut s1_signed = [0i16; FALCON_512_N];
    for i in 0..FALCON_512_N {
        s1_signed[i] = s1.coeffs[i].balanced_value();
    }
    
    // compute L2 norm squared: ||s1||^2 + ||s2||^2
    let mut norm_squared_fixed = 0i64;
    
    // adding ||s1||^2
    for i in 0..FALCON_512_N {
        let s1_val = s1_signed[i] as i64;
        norm_squared_fixed += s1_val * s1_val * FIXED_POINT_SCALE;
    }
    
    // adding ||s2||^2
    for i in 0..FALCON_512_N {
        let s2_val = s2_coeffs[i] as i64;
        norm_squared_fixed += s2_val * s2_val * FIXED_POINT_SCALE;
    }
    
    // signature bound
    if norm_squared_fixed >= FALCON_512_SIG_BOUND_FIXED {
        return Err(ProgramError::InvalidAccountData);
    }
    
    Ok(())
}

// NTT (Number Theoretic Transform) operation
// on mainnet, this would perform the actual NTT transformation
#[allow(dead_code)]
fn ntt_forward(_coeffs: &mut [u32; FALCON_512_N]) {
    // TODO: implement forward NTT
    // this involves bit-reversal and butterfly operations
    // so we're using precomputed twiddle factors
}

// Inverse NTT operation
#[allow(dead_code)]
fn ntt_inverse(_coeffs: &mut [u32; FALCON_512_N]) {
    // TODO: Implement inverse NTT
    // this is similar to forward NTT but with different twiddle factors
    // and a final scaling step
}

// modular reduction... modulo q = 12289
#[allow(dead_code)]
fn mod_q(x: u32) -> u32 {
    // barrett reduction for q = 12289
    const Q: u32 = 12289;
    const BARRETT_SHIFT: u32 = 26;
    const BARRETT_MULTIPLIER: u32 = 5467;
    
    let t = ((x as u64 * BARRETT_MULTIPLIER as u64) >> BARRETT_SHIFT) as u32;
    let r = x - t * Q;
    
    if r >= Q { r - Q } else { r }
} 