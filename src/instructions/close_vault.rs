use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};
use crate::falcon::{FalconSignature, FalconPublicKey, FALCON_512_SIGNATURE_SIZE, FALCON_512_PUBLIC_KEY_SIZE};

pub struct CloseVault {
    signature: FalconSignature,
    bump: u8,
}

impl CloseVault {
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProgramError> {
        let expected_size = FALCON_512_SIGNATURE_SIZE + 1;
        if bytes.len() != expected_size {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut signature_bytes = [0u8; FALCON_512_SIGNATURE_SIZE];
        signature_bytes.copy_from_slice(&bytes[0..FALCON_512_SIGNATURE_SIZE]);
        let bump = bytes[FALCON_512_SIGNATURE_SIZE];

        Ok(Self {
            signature: FalconSignature::from(signature_bytes),
            bump,
        })
    }

    pub fn process(&self, accounts: &[AccountInfo]) -> ProgramResult {
        // asert we have exactly 2 accounts
        let [vault, refund] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // check that vault is owned by our program
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

        // create message to verify
        // Message: "CLOSE_VAULT" + refund pubkey
        let mut message = [0u8; 43];
        message[0..11].copy_from_slice(b"CLOSE_VAULT");
        message[11..43].copy_from_slice(refund.key());

        // verify the Falcon signature
        self.signature.verify(&public_key, &message)?;

        // Verify PDA
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

        // close vault and refund all lamports to refund account
        *refund.try_borrow_mut_lamports()? += vault.lamports();
        vault.close()
    }
} 