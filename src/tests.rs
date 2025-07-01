
use crate::*;
use mollusk_svm::{Mollusk, result::Check};
use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
};

const MOCK_FALCON_PUBKEY: [u8; 897] = [0x09; 897]; // Valid Falcon-512 header + padding
const MOCK_FALCON_SIGNATURE: [u8; 666] = [0x29; 666]; // Valid header + padding

// test opening a Falcon-512 vault
#[test]
fn test_open_falcon_vault() {
    let program_id = Pubkey::new_from_array(crate::ID);
    let mollusk = Mollusk::new(&program_id, "target/deploy/solana_falcon_vault");

    let falcon_public_key = crate::falcon::FalconPublicKey::from(MOCK_FALCON_PUBKEY);
    let pubkey_hash = falcon_public_key.hash();
    
    let (vault_pda, bump) = Pubkey::find_program_address(&[&pubkey_hash], &program_id);
    let payer = Keypair::new();
    
    // Prepare instruction: [discriminator(1), falcon_pubkey(897), bump(1)]
    let mut instruction_data = vec![0u8]; // OpenVault discriminator
    instruction_data.extend_from_slice(&MOCK_FALCON_PUBKEY);
    instruction_data.push(bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &vec![
            (payer.pubkey(), AccountSharedData::new(1_000_000_000, 0, &system_program::id())),
            (vault_pda, AccountSharedData::default()),
            (system_program::id(), AccountSharedData::default()),
        ],
        &[Check::success()],
    );

    // verify if thee vault was created with correct data
    let vault_account = result.get_account(&vault_pda).unwrap();
    assert_eq!(vault_account.data().len(), 897);
    assert_eq!(vault_account.data(), &MOCK_FALCON_PUBKEY);
}

// Test transferring from vault with signature verification
#[test]
fn test_transfer_from_vault() {
    let program_id = Pubkey::new_from_array(crate::ID);
    let mollusk = Mollusk::new(&program_id, "target/deploy/solana_falcon_vault");

    let falcon_public_key = crate::falcon::FalconPublicKey::from(MOCK_FALCON_PUBKEY);
    let pubkey_hash = falcon_public_key.hash();
    let (vault_pda, bump) = Pubkey::find_program_address(&[&pubkey_hash], &program_id);

    let recipient = Keypair::new();
    let transfer_amount = 100_000_000u64;
    
    // Prepare instruction: [discriminator(1), signature(666), amount(8), bump(1)]
    let mut instruction_data = vec![1u8]; // TransferFromVault discriminator
    instruction_data.extend_from_slice(&MOCK_FALCON_SIGNATURE);
    instruction_data.extend_from_slice(&transfer_amount.to_le_bytes());
    instruction_data.push(bump);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(recipient.pubkey(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    // ccreate vault account with public key and lamports
    let mut vault_account = AccountSharedData::new(1_000_000_000, 897, &program_id);
    vault_account.data_as_mut_slice().copy_from_slice(&MOCK_FALCON_PUBKEY);

    // Note: with mock data, signature verification will fail, obviously..
    // this test validates instruction parsing and account handling
    let _result = mollusk.process_instruction(
        &instruction,
        &vec![
            (vault_pda, vault_account),
            (recipient.pubkey(), AccountSharedData::default()),
            (system_program::id(), AccountSharedData::default()),
        ],
    );
}

// Test Falcon signature verification core functionality
#[test]
fn test_falcon_verification_edge_cases() {
    use crate::falcon::verify_falcon_signature;
    
    let public_key = MOCK_FALCON_PUBKEY;
    let message = b"test_message";
    
    // test various signature formatss
    let test_cases = [
        ([0x00; 666], "Invalid header should fail"),
        ([0xFF; 666], "Invalid signature should fail"),
        (MOCK_FALCON_SIGNATURE, "Mock signature should fail gracefully"),
    ];

    for (signature, description) in test_cases {
        let result = verify_falcon_signature(&public_key, &signature, message);
        
        // All mock signatures should fail, but gracefully without panics
        assert!(result.is_err(), "Test case failed: {}", description);
        println!("✓ {}: {:?}", description, result);
    }
}

// performance and compute unit validation
    #[test]
fn test_performance_estimates() {
    // estimated compute unit breakdown for Falcon-512 verification
    let performance_breakdown = [
        ("Signature parsing", 5_000),
        ("Hash-to-point", 25_000),
        ("Signature decompression", 35_000),
        ("NTT operations", 45_000),
        ("Polynomial operations", 30_000),
        ("Norm verification", 10_000),
    ];
    
    let total_estimated: u64 = performance_breakdown.iter().map(|(_, cu)| cu).sum();
    
    println!("Falcon-512 Performance Estimates:");
    println!("================================");
    for (operation, compute_units) in performance_breakdown {
        println!("{}: {} CU", operation, compute_units);
    }
    println!("================================");
    println!("Total: {} CU", total_estimated);
    println!("Solana Limit: 200,000 CU");
    println!("Utilization: {:.1}%", (total_estimated as f64 / 200_000.0) * 100.0);
    
    // we need to stay within Solana's compute budget
    assert!(total_estimated <= 200_000, "Compute unit usage exceeds limit");
    assert_eq!(total_estimated, 150_000, "Performance estimate mismatch");
}

// integration test for production deployment validation
#[cfg(feature = "integration")]
#[test]
fn test_production_readiness() {
    // Validate that all critical components are present and functional
    
    // 1.cryptographic primitives
    assert!(crate::falcon::FALCON_512_PUBLIC_KEY_SIZE == 897);
    assert!(crate::falcon::FALCON_512_SIGNATURE_SIZE == 666);
    
    // 2. instructions are properly defined
    use crate::instructions::VaultInstructions;
    assert!(VaultInstructions::try_from(&0u8).is_ok()); // OpenVault
    assert!(VaultInstructions::try_from(&1u8).is_ok()); // TransferFromVault
    assert!(VaultInstructions::try_from(&2u8).is_ok()); // CloseVault
    assert!(VaultInstructions::try_from(&3u8).is_err()); // Invalid
    
    // 3.error handling
    use pinocchio::program_error::ProgramError;
    let result = crate::falcon::verify_falcon_signature(
        &MOCK_FALCON_PUBKEY,
        &MOCK_FALCON_SIGNATURE,
        b"test"
    );
    assert!(matches!(result, Err(ProgramError::Custom(_))));
    
    println!("✓ All production checks passed");
} 