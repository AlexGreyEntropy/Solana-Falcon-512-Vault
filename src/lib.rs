#![allow(unexpected_cfgs)]

pub mod instructions;
use instructions::*;

pub mod falcon;

#[cfg(test)]
pub mod tests;

use pinocchio::{
    account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey,
    ProgramResult,
};

// Program ID... update this with your deployed program ID
// generated using: solana-keygen new --outfile program-keypair.json
pub const ID: Pubkey = [
    0x39, 0x65, 0xE5, 0x2C, 0x78, 0x96, 0xF7, 0x4E, 
    0x95, 0x25, 0x8F, 0x52, 0xB6, 0xFB, 0x0D, 0x47,
    0x35, 0x23, 0xA8, 0xED, 0x52, 0x88, 0x91, 0x71, 
    0x8C, 0x36, 0x4F, 0xB2, 0x9A, 0x7E, 0x6D, 0x41,
];

entrypoint!(process_instruction);

// Main program entry point

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    
    match VaultInstructions::try_from(discriminator)? {
        VaultInstructions::OpenVault => {
            OpenVault::deserialize(data)?.process(accounts, program_id)
        },
        VaultInstructions::TransferFromVault => {
            TransferFromVault::deserialize(data)?.process(accounts)
        },
        VaultInstructions::CloseVault => {
            CloseVault::deserialize(data)?.process(accounts)
        },
    }
} 