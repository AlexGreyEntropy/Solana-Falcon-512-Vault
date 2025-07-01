// SHAKE256 implementation for Falcon-512 verification
// Keccak-f[1600] permutation for no_std environments

// Keccak state size in 64-bit words
const STATE_SIZE: usize = 25;

// SHAKE256 rate in bytes (1600 - 256*2) / 8 = 136
const SHAKE256_RATE: usize = 136;

// round constants for Keccak-f[1600]
const ROUND_CONSTANTS: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
    0x8000000000000000, 0x8000000080008082, 0x800000000000808a, 0x8000000080000000,
];

// rotation offsets for rho step
const RHO_OFFSETS: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];

// SHAKE256 hasher state
pub struct Shake256 {
    state: [u64; STATE_SIZE],
    buffer: [u8; SHAKE256_RATE],
    buffer_len: usize,
    absorbed: bool,
}

impl Shake256 {
    // this creates a new SHAKE256 hasher
    pub fn new() -> Self {
        Self {
            state: [0u64; STATE_SIZE],
            buffer: [0u8; SHAKE256_RATE],
            buffer_len: 0,
            absorbed: false,
        }
    }

    // absorb input data
    pub fn update(&mut self, data: &[u8]) {
        if self.absorbed {
            panic!("Cannot update after finalization");
        }

        let mut offset = 0;
        while offset < data.len() {
            let take = core::cmp::min(SHAKE256_RATE - self.buffer_len, data.len() - offset);
            
            self.buffer[self.buffer_len..self.buffer_len + take]
                .copy_from_slice(&data[offset..offset + take]);
            
            self.buffer_len += take;
            offset += take;

            if self.buffer_len == SHAKE256_RATE {
                self.absorb_block();
                self.buffer_len = 0;
            }
        }
    }

    // finalize absorption and return a reader for squeezing
    pub fn finalize_xof(mut self) -> Shake256Reader {
        // SHAKE256 padding: append 0x1f and pad to rate
        self.buffer[self.buffer_len] = 0x1f;
        for i in self.buffer_len + 1..SHAKE256_RATE {
            self.buffer[i] = 0;
        }
        // setting the last bit for domain separation
        self.buffer[SHAKE256_RATE - 1] |= 0x80;
        
        self.absorb_block();
        self.absorbed = true;

        Shake256Reader {
            state: self.state,
            buffer: [0u8; SHAKE256_RATE],
            buffer_len: 0,
        }
    }

    // absorb a rate-sized block into the state
    fn absorb_block(&mut self) {
        // XOR buffer into state (little-endian interpretation - 64 bits at a time)
        for i in 0..SHAKE256_RATE / 8 {
            let mut lane = 0u64;
            for j in 0..8 {
                lane |= (self.buffer[i * 8 + j] as u64) << (j * 8);
            }
            self.state[i] ^= lane;
        }

        // apply the Keccak-f[1600] permutation
        keccak_f1600(&mut self.state);
    }
}

// reader for squeezing output from SHAKE256
pub struct Shake256Reader {
    state: [u64; STATE_SIZE],
    buffer: [u8; SHAKE256_RATE],
    buffer_len: usize,
}

impl Shake256Reader {
    // read output bytes from the SHAKE256 XOF
    pub fn read(&mut self, output: &mut [u8]) {
        let mut offset = 0;
        
        while offset < output.len() {
            if self.buffer_len == 0 {
                self.squeeze_block();
                self.buffer_len = SHAKE256_RATE;
            }

            let take = core::cmp::min(self.buffer_len, output.len() - offset);
            let start = SHAKE256_RATE - self.buffer_len;
            
            output[offset..offset + take]
                .copy_from_slice(&self.buffer[start..start + take]);
            
            offset += take;
            self.buffer_len -= take;
        }
    }

    // squeeze a rate-sized block from the state
    fn squeeze_block(&mut self) {
        
        for i in 0..SHAKE256_RATE / 8 {
            let lane = self.state[i];
            for j in 0..8 {
                self.buffer[i * 8 + j] = (lane >> (j * 8)) as u8;
            }
        }

        //apply Keccak-f[1600] permutation for next block
        keccak_f1600(&mut self.state);
    }
}

// Keccak-f[1600] permutation function
// implementation of the 24-round Keccak permutation
fn keccak_f1600(state: &mut [u64; STATE_SIZE]) {
    for round in 0..24 {
        // θ (Theta) step
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }

        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }

        for x in 0..5 {
            for y in 0..5 {
                state[y * 5 + x] ^= d[x];
            }
        }

        // ρ (Rho) and π (Pi) steps combined
        let mut current = state[1];
        for t in 0..24 {
            let (x, y) = pi_coordinates(t);
            let temp = state[y * 5 + x];
            state[y * 5 + x] = current.rotate_left(RHO_OFFSETS[t]);
            current = temp;
        }

        // χ (Chi) step
        for y in 0..5 {
            let mut row = [0u64; 5];
            for x in 0..5 {
                row[x] = state[y * 5 + x];
            }
            for x in 0..5 {
                state[y * 5 + x] = row[x] ^ ((!row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }

        // ι (Iota) step
        state[0] ^= ROUND_CONSTANTS[round];
    }
}

// compute the Pi step coordinates
#[inline]
fn pi_coordinates(t: usize) -> (usize, usize) {
    let mut x = 1;
    let mut y = 0;
    for _ in 0..t {
        let temp_x = x;
        x = y;
        y = (2 * temp_x + 3 * y) % 5;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shake256_empty() {
        let mut hasher = Shake256::new();
        let mut reader = hasher.finalize_xof();
        
        let mut output = [0u8; 32];
        reader.read(&mut output);
        
        //test vector for SHAKE256("")
        let expected = [
            0x46, 0xb9, 0xdd, 0x2b, 0x0b, 0xa8, 0x8d, 0x13,
            0x23, 0x3b, 0x3f, 0xeb, 0x74, 0x3e, 0xeb, 0x24,
            0x3f, 0xcd, 0x52, 0xea, 0x62, 0xb8, 0x1b, 0x82,
            0xb5, 0x0c, 0x27, 0x64, 0x6e, 0xd5, 0x76, 0x2f,
        ];
        
        assert_eq!(output, expected);
    }

    #[test]
    fn test_shake256_abc() {
        let mut hasher = Shake256::new();
        hasher.update(b"abc");
        let mut reader = hasher.finalize_xof();
        
        let mut output = [0u8; 32];
        reader.read(&mut output);
        
        // test vector for SHAKE256("abc")
        let expected = [
            0x48, 0x33, 0x66, 0x60, 0x13, 0x60, 0xa8, 0x77,
            0x1c, 0x68, 0x63, 0x08, 0x0c, 0xc4, 0x11, 0x4d,
            0x8d, 0xb4, 0x45, 0x30, 0xf8, 0xf1, 0xe1, 0xee,
            0x4f, 0x94, 0xea, 0x37, 0xe7, 0x8b, 0x57, 0x39,
        ];
        
        assert_eq!(output, expected);
    }

    #[test]
    fn test_multiple_reads() {
        let mut hasher = Shake256::new();
        hasher.update(b"test");
        let mut reader = hasher.finalize_xof();
        
        let mut output1 = [0u8; 16];
        let mut output2 = [0u8; 16];
        
        reader.read(&mut output1);
        reader.read(&mut output2);
        
        // this should produce different outputs (continuous stream, not just the same output)
        assert_ne!(output1, output2);
    }
} 