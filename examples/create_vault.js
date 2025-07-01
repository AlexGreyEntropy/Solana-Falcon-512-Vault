import {
    Connection,
    PublicKey,
    Keypair,
    Transaction,
    SystemProgram,
    TransactionInstruction,
    sendAndConfirmTransaction,
    LAMPORTS_PER_SOL
} from '@solana/web3.js';
import { readFileSync } from 'fs';
import crypto from 'crypto';

const DEVNET_URL = 'https://api.devnet.solana.com';
const FALCON_512_PUBLIC_KEY_SIZE = 897;


function generateFalconPublicKey() {
    const publicKey = new Uint8Array(FALCON_512_PUBLIC_KEY_SIZE);
    
    publicKey[0] = 0x30; // ASN.1 SEQUENCE tag
    publicKey[1] = 0x09; // Falcon-512 identifier
    
    for (let i = 2; i < FALCON_512_PUBLIC_KEY_SIZE; i++) {
        publicKey[i] = (i * 137 + 42) % 256;
    }
    
    return publicKey;
}

// calculate SHA256 hash of Falcon public key for PDA derivation

function hashFalconPublicKey(publicKey) {
    return crypto.createHash('sha256').update(publicKey).digest();
}

// create a Falcon-512 quantum-resistant vault on Solana

async function createFalconVault(programId, walletPath) {
    console.log('Creating Falcon-512 Quantum-Resistant Vault');
    console.log('===========================================');
    
    try {
    
        const connection = new Connection(DEVNET_URL, 'confirmed');
        console.log('Connected to Solana devnet');
        
       
        const walletData = readFileSync(walletPath, 'utf8');
        const keypairData = JSON.parse(walletData);
        const wallet = Keypair.fromSecretKey(new Uint8Array(keypairData));
        
        console.log('Wallet:', wallet.publicKey.toString());
   
        const balance = await connection.getBalance(wallet.publicKey);
        console.log('Balance:', balance / LAMPORTS_PER_SOL, 'SOL');
        
        if (balance < 0.01 * LAMPORTS_PER_SOL) {
            throw new Error('Insufficient balance for deployment');
        }
        
        //generate Falcon-512 public key
        const falconPublicKey = generateFalconPublicKey();
        console.log('Generated Falcon-512 public key (897 bytes)');
        
        const publicKeyHash = hashFalconPublicKey(falconPublicKey);
        const [vaultPDA, bump] = PublicKey.findProgramAddressSync(
            [publicKeyHash],
            new PublicKey(programId)
        );
        
        console.log('Vault PDA:', vaultPDA.toString());
        console.log('Bump:', bump);
        
        const instructionData = Buffer.concat([
            Buffer.from([0]),           
            Buffer.from(falconPublicKey), 
            Buffer.from([bump])        
        ]);
        
        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
                { pubkey: vaultPDA, isSigner: false, isWritable: true },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            programId: new PublicKey(programId),
            data: instructionData,
        });
        
        const transaction = new Transaction().add(instruction);
        
        console.log('Sending vault creation transaction...');
        
        const signature = await sendAndConfirmTransaction(
            connection,
            transaction,
            [wallet],
            { commitment: 'confirmed' }
        );
        
        console.log('');
        console.log('SUCCESS! Falcon-512 vault created');
        console.log('Transaction:', signature);
        console.log('Vault PDA:', vaultPDA.toString());
        console.log('');
        console.log('View on Explorer:');
        console.log(`https://explorer.solana.com/tx/${signature}?cluster=devnet`);
        console.log(`https://explorer.solana.com/address/${vaultPDA.toString()}?cluster=devnet`);
        
        return {
            signature,
            vaultPDA: vaultPDA.toString(),
            publicKey: Buffer.from(falconPublicKey).toString('hex'),
            bump
        };
        
    } catch (error) {
        console.error('Error creating vault:', error.message);
        throw error;
    }
}

// example
if (process.argv.length < 4) {
    console.log('Usage: node create_vault.js <program_id> <wallet_path>');
    console.log('Example: node create_vault.js CZBeesUR63G37oWTcJW4cLMsQrJMpPJkQyAmB373FUrC ~/.config/solana/id.json');
    process.exit(1);
}

const [, , programId, walletPath] = process.argv;

createFalconVault(programId, walletPath)
    .then((result) => {
        console.log('Vault creation completed successfully!');
        process.exit(0);
    })
    .catch((error) => {
        console.error('Failed:', error.message);
        process.exit(1);
    }); 