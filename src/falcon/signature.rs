use pinocchio::program_error::ProgramError;
use crate::falcon::verify::{FALCON_512_PUBLIC_KEY_SIZE, FALCON_512_SIGNATURE_SIZE};

// Falcon-512 public key representation
#[derive(Clone, Copy)]
pub struct FalconPublicKey {
    pub bytes: [u8; FALCON_512_PUBLIC_KEY_SIZE],
}

impl FalconPublicKey {
    pub fn new(bytes: [u8; FALCON_512_PUBLIC_KEY_SIZE]) -> Self {
        Self { bytes }
    }
    
    // hash the public key to create a seed for PDA
    // using SHA256 to be compatible with Solana's PDA derivation
    pub fn hash(&self) -> [u8; 32] {
        solana_nostd_sha256::hash(&self.bytes).into()
    }
}

// Falcon-512 signature representation
#[derive(Clone, Copy)]
pub struct FalconSignature {
    pub bytes: [u8; FALCON_512_SIGNATURE_SIZE],
}

impl FalconSignature {
    pub fn new(bytes: [u8; FALCON_512_SIGNATURE_SIZE]) -> Self {
        Self { bytes }
    }
    
    // verify a signature against a public key and message
    pub fn verify(&self, public_key: &FalconPublicKey, message: &[u8]) -> Result<(), ProgramError> {
        // using the verification function
        crate::falcon::verify::verify_falcon_signature(
            &public_key.bytes,
            &self.bytes,
            message
        )
    }
}

impl From<[u8; FALCON_512_SIGNATURE_SIZE]> for FalconSignature {
    fn from(bytes: [u8; FALCON_512_SIGNATURE_SIZE]) -> Self {
        Self { bytes }
    }
}

impl From<[u8; FALCON_512_PUBLIC_KEY_SIZE]> for FalconPublicKey {
    fn from(bytes: [u8; FALCON_512_PUBLIC_KEY_SIZE]) -> Self {
        Self { bytes }
    }
} 