use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use crate::falcon::{FalconSignature, FalconPublicKey, FALCON_512_SIGNATURE_SIZE, FALCON_512_PUBLIC_KEY_SIZE};

pub struct TransferFromVault {
    signature: FalconSignature,
    amount: u64,
    bump: u8,
}

impl TransferFromVault {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProgramError> {
        let expected_size = FALCON_512_SIGNATURE_SIZE + 8 + 1;
        if bytes.len() != expected_size {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut signature_bytes = [0u8; FALCON_512_SIGNATURE_SIZE];
        signature_bytes.copy_from_slice(&bytes[0..FALCON_512_SIGNATURE_SIZE]);
        
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[FALCON_512_SIGNATURE_SIZE..FALCON_512_SIGNATURE_SIZE + 8]);
        
        let bump = bytes[FALCON_512_SIGNATURE_SIZE + 8];

        Ok(Self {
            signature: FalconSignature::from(signature_bytes),
            amount: u64::from_le_bytes(amount_bytes),
            bump,
        })
    }

    pub fn process(&self, accounts: &[AccountInfo]) -> ProgramResult {
        // assert we have exactly 3 accounts
        let [vault, recipient, _system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // check that vault is owned by our programm
        // AccountInfo::owner() is safe to call as it's just reading the account's owner field
        if unsafe { vault.owner() } != &crate::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        // read the public key from the vault account
        let vault_data = vault.try_borrow_data()?;
        if vault_data.len() != FALCON_512_PUBLIC_KEY_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }
        
        let mut public_key_bytes = [0u8; FALCON_512_PUBLIC_KEY_SIZE];
        public_key_bytes.copy_from_slice(&vault_data);
        let public_key = FalconPublicKey::from(public_key_bytes);
        drop(vault_data);

        // Create the message to verify
        // message includes: amount (8 bytes) + recipient pubkey (32 bytes) + current slot (8 bytes)
        let mut message = [0u8; 48];
        message[0..8].copy_from_slice(&self.amount.to_le_bytes());
        message[8..40].copy_from_slice(recipient.key());
        // on mainnet, we would include the current slot or nonce for replay protection
        // for now... we'll use a placeholder
        message[40..48].copy_from_slice(&[0u8; 8]);

        // verify the Falcon signature
        self.signature.verify(&public_key, &message)?;

        // verify PDA (similar to Winternitz vault, thanks Dean!)
        let pubkey_hash = public_key.hash();
        if solana_nostd_sha256::hashv(&[
            pubkey_hash.as_ref(),
            &[self.bump],
            crate::ID.as_ref(),
            b"ProgramDerivedAddress",
        ])
        .ne(vault.key())
        {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // check vault has sufficient balance
        if vault.lamports() < self.amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // trasfer lamports from vault to recipient
        *vault.try_borrow_mut_lamports()? -= self.amount;
        *recipient.try_borrow_mut_lamports()? += self.amount;

        Ok(())
    }
} 