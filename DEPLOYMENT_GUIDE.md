# Falcon-512 Vault Deployment Guide

This guide covers deploying the Falcon-512 quantum-resistant vault program to Solana.

## **Prerequisites**

### **Required Tools**
- **Rust** 1.80+ with `cargo-build-sbf`
- **Solana CLI** 1.18+
- **Node.js** 18+ (for client examples)

### **Installation**

```bash
# install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="~/.local/share/solana/install/active_release/bin:$PATH"

# install Solana BPF toolchain
cargo install cargo-build-sbf
```

## **Deployment Steps**

### **1. Clone and Build**

```bash
# clone the repository
git clone https://github.com/your-org/solana-falcon-512-vaults
cd solana-falcon-512-vaults

# Install JavaScript dependencies
npm install

# build program
cargo build-sbf
```

### **2. Setup Solana Environment**

```bash
# configure for devnet
solana config set --url devnet

# Create or import wallet
solana-keygen new --outfile ~/.config/solana/id.json

# devnet SOL (for testing)
solana airdrop 2

# Check balance
solana balance
```

### **3. Deploy Program**

```bash
# deploy to devnet
solana program deploy target/deploy/solana_falcon_vault.so

# note the Program ID from output... you'll need this for client
```

### **4. Verify Deployment**

```bash
# check program account
solana account <PROGRAM_ID>

# Verify program is executable
solana program show <PROGRAM_ID>
```

## **Testing the Deployment**

### **Create a Test Vault**

```bash
# use example client
node examples/create_vault.js <PROGRAM_ID> ~/.config/solana/id.json
```



### **Mainnet Deployment**

```bash
# switch to mainnet-beta
solana config set --url mainnet-beta

# sufficient SOL for deployment? (~0.5 SOL)
solana balance

# Deploy to mainnet
solana program deploy target/deploy/solana_falcon_vault.so
```

### **Security Considerations**

1. **Key Management**: Use hardware wallets for mainnet deployments
2. **Program Verification**: Verify program hash matches expected binary
3. **Access Control**: Implement proper upgrade authority management
4. **Monitoring**: Set up alerts for program interactions

## **Troubleshooting**

### **Common Issues**

**Build Failures:**
```bash
# If cargo-build-sbf is missing
cargo install cargo-build-sbf

# if dependencies fail
cargo clean && cargo build-sbf
```

**Deployment Failures:**
```bash
# Check Solana CLI version
solana --version

# verify network connection
solana cluster-version

# or wallet balance
solana balance
```

**Transaction Failures:**
- wallet has sufficient SOL
- Verify program ID is correct
- Check instruction data format

## **Performance Metrics**

### **Deployment Costs**
- **Devnet**: Free (with airdrop)
- **Mainnet**: ~0.5 SOL for initial deployment

### **Transaction Costs**
- **Vault Creation**: ~0.001 SOL
- **Vault Transfer**: ~0.0005 SOL
- **Vault Closure**: ~0.0005 SOL

### **Compute Unit Usage**
- **OpenVault**: ~152,200 CU (76% of limit)
- **TransferFromVault**: ~155,000 CU (78% of limit)
- **CloseVault**: ~25,000 CU (12% of limit)

---