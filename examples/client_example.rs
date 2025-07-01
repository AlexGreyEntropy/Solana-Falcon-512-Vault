use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

const FALCON_512_PUBLIC_KEY_SIZE: usize = 897;
const FALCON_512_SIGNATURE_SIZE: usize = 666;

fn main() {
    
    let falcon_public_key = generate_mock_falcon_public_key();
    let falcon_private_key = generate_mock_falcon_private_key();
    
    let program_id = Pubkey::new_unique(); // Your program ID
    let pubkey_hash = hash_falcon_public_key(&falcon_public_key);
    let (vault_pda, bump) = Pubkey::find_program_address(
        &[&pubkey_hash],
        &program_id,
    );
    
    println!("Vault PDA: {}", vault_pda);
    

    let payer = Keypair::new();
    let open_vault_ix = create_open_vault_instruction(
        &program_id,
        &payer.pubkey(),
        &vault_pda,
        &falcon_public_key,
        bump,
    );
    
    let recipient = Keypair::new();
    let transfer_amount = 100_000_000; // 0.1 SOL
    
    let mut transfer_message = vec![0u8; 48];
    transfer_message[0..8].copy_from_slice(&transfer_amount.to_le_bytes());
    transfer_message[8..40].copy_from_slice(&recipient.pubkey().to_bytes());
    transfer_message[40..48].copy_from_slice(&[0u8; 8]); // nonce placeholder
    
    let transfer_signature = sign_with_falcon(&falcon_private_key, &transfer_message);
    
    let transfer_ix = create_transfer_instruction(
        &program_id,
        &vault_pda,
        &recipient.pubkey(),
        transfer_amount,
        &transfer_signature,
        bump,
    );
    
    let refund_account = Keypair::new();
    
    let mut close_message = vec![0u8; 43];
    close_message[0..11].copy_from_slice(b"CLOSE_VAULT");
    close_message[11..43].copy_from_slice(&refund_account.pubkey().to_bytes());
    
    let close_signature = sign_with_falcon(&falcon_private_key, &close_message);
    
    let close_ix = create_close_vault_instruction(
        &program_id,
        &vault_pda,
        &refund_account.pubkey(),
        &close_signature,
        bump,
    );
    
    println!("Example instructions created successfully!");
}


fn generate_mock_falcon_public_key() -> [u8; FALCON_512_PUBLIC_KEY_SIZE] {
    let mut key = [0u8; FALCON_512_PUBLIC_KEY_SIZE];
    key[0..32].copy_from_slice(&[1u8; 32]);
    key
}

fn generate_mock_falcon_private_key() -> Vec<u8> {
    vec![2u8; 2048]
}

fn hash_falcon_public_key(public_key: &[u8; FALCON_512_PUBLIC_KEY_SIZE]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    public_key.hash(&mut hasher);
    let hash_value = hasher.finish();
    
    let mut result = [0u8; 32];
    result[0..8].copy_from_slice(&hash_value.to_le_bytes());
    result
}

fn sign_with_falcon(_private_key: &[u8], _message: &[u8]) -> [u8; FALCON_512_SIGNATURE_SIZE] {
    let mut signature = [0u8; FALCON_512_SIGNATURE_SIZE];
    signature[0] = 0xFF; 
    signature
}

fn create_open_vault_instruction(
    program_id: &Pubkey,
    payer: &Pubkey,
    vault_pda: &Pubkey,
    falcon_public_key: &[u8; FALCON_512_PUBLIC_KEY_SIZE],
    bump: u8,
) -> Instruction {
    let mut data = vec![0u8]; 
    data.extend_from_slice(falcon_public_key);
    data.push(bump);
    
    Instruction::new_with_bytes(
        *program_id,
        &data,
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
    )
}

fn create_transfer_instruction(
    program_id: &Pubkey,
    vault_pda: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
    signature: &[u8; FALCON_512_SIGNATURE_SIZE],
    bump: u8,
) -> Instruction {
    let mut data = vec![1u8]; 
    data.extend_from_slice(signature);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(bump);
    
    Instruction::new_with_bytes(
        *program_id,
        &data,
        vec![
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(*recipient, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
    )
}

fn create_close_vault_instruction(
    program_id: &Pubkey,
    vault_pda: &Pubkey,
    refund: &Pubkey,
    signature: &[u8; FALCON_512_SIGNATURE_SIZE],
    bump: u8,
) -> Instruction {
    let mut data = vec![2u8]; 
    data.extend_from_slice(signature);
    data.push(bump);
    
    Instruction::new_with_bytes(
        *program_id,
        &data,
        vec![
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(*refund, false),
        ],
    )
} 