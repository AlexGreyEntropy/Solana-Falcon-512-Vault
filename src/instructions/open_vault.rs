use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use crate::falcon::{FalconPublicKey, FALCON_512_PUBLIC_KEY_SIZE};

pub struct OpenVault {
    public_key: FalconPublicKey,
    bump: u8,
}

impl OpenVault {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProgramError> {
        let expected_size = FALCON_512_PUBLIC_KEY_SIZE + 1;
        if bytes.len() != expected_size {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut pubkey_bytes = [0u8; FALCON_512_PUBLIC_KEY_SIZE];
        pubkey_bytes.copy_from_slice(&bytes[0..FALCON_512_PUBLIC_KEY_SIZE]);
        let bump = bytes[FALCON_512_PUBLIC_KEY_SIZE];
        
        Ok(Self {
            public_key: FalconPublicKey::from(pubkey_bytes),
            bump,
        })
    }

    pub fn process(&self, accounts: &[AccountInfo], program_id: &pinocchio::pubkey::Pubkey) -> ProgramResult {
        // assert we have exactly 3 accounts
        let [payer, vault, _system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Hash the Falcon public key to create a 32-byte seed for the PDA
        let pubkey_hash = self.public_key.hash();
        let bump_array = [self.bump];
        
        // Standard Solana PDA: [seed, bump] using actual program_id
        let seeds = [Seed::from(&pubkey_hash), Seed::from(&bump_array)];
        
        // rent for storing the public key
        let lamports = Rent::get()?.minimum_balance(FALCON_512_PUBLIC_KEY_SIZE);
        
        let signers = [Signer::from(&seeds)];

        // create vault with space for the public key
        CreateAccount {
            from: payer,
            to: vault,
            lamports,
            space: FALCON_512_PUBLIC_KEY_SIZE as u64,
            owner: program_id,
        }
        .invoke_signed(&signers[..])?;
        
        // store the public key in the vault account
        vault.try_borrow_mut_data()?
            .copy_from_slice(&self.public_key.bytes);
        
        Ok(())
    }
} 