use pinocchio::program_error::ProgramError;

pub enum VaultInstructions {
    OpenVault,
    TransferFromVault,
    CloseVault,
}

impl TryFrom<&u8> for VaultInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::OpenVault),
            1 => Ok(Self::TransferFromVault),
            2 => Ok(Self::CloseVault),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
} 