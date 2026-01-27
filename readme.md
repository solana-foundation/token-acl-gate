# Allow Block List (ABL) - sRFC 37 Implementation

A Solana program implementation of sRFC 37 that provides an Allow/Block List (ABL) system for token access control. This program implements the `can_thaw_permissionless` instruction according to the sRFC037 specification.

## Overview

This ABL program is written in Pinocchio and generates clients using Codama. It provides a flexible token access control system that allows token issuers to manage which wallets can have their token accounts thawed through different operational modes.

## Repo contents

- **Program**: Core Solana program written in Pinocchio
- **SDK**: Generated TypeScript and Rust clients using Codama
- **CLI**: Command-line interface for program interaction

## Working Modes

The program supports three distinct operational modes:

### 1. Block Mode
- **Purpose**: Blocks wallets in the list from having token accounts thawed
- **Behavior**: Only wallets NOT in the list can have their token accounts thawed
- **Use Case**: Blacklist approach for preventing specific wallets from accessing tokens

### 2. Allow Mode  
- **Purpose**: Only wallets in the list can have their token accounts thawed
- **Behavior**: Wallets must be explicitly added to the list to access tokens
- **Use Case**: Whitelist approach for restricted token access

### 3. Allow All EOAs Mode
- **Purpose**: All EOAs can be thawed by default, plus any manually added wallets
- **Behavior**: 
  - All EOAs are allowed by default
  - PDAs (Program Derived Addresses) must be explicitly added to the list
- **Use Case**: Open access for EOAs with selective PDA (Smart Contracts) control

## Core Functionality

### List Management
- **Create List**: Initialize a new allow/block list with specified mode
- **Delete List**: Remove an existing list (only when empty)
- **Add Wallet**: Add a wallet address to a specific list
- **Remove Wallet**: Remove a wallet address from a specific list

### Token Integration
- **Setup Extra Metas**: Configure which lists are used for a given token mint
- **Multiple List Support**: Token issuers can subscribe to multiple allow or block lists
- **Conjunctive Logic**: Wallets must be allowed by ALL configured lists to be thawed

## Program Instructions

| Instruction | Discriminator | Description |
|-------------|---------------|-------------|
| `can_thaw_permissionless` | `0x8` | Gate program instruction called by Token ACL |
| `create_list` | `0x1` | Create a new list configuration |
| `delete_list` | `0x2` | Delete an empty list |
| `add_wallet` | `0x3` | Add wallet to a list |
| `remove_wallet` | `0x4` | Remove wallet from a list |
| `setup_extra_metas` | `0x5` | Configure lists for a token mint |

## Integration with Token ACL

This program serves as a gate program for the [Token ACL system](https://github.com/solana-foundation/token-acl). The Token ACL program calls `can_thaw_permissionless` to determine if a wallet should be allowed to thaw their token account.

### Setup Process
1. Create one or more lists with desired modes
2. Add/remove wallets as needed
3. Use `setup_extra_metas` to configure which lists apply to a token mint
4. Create a Token ACL mint config account and define this program as the gate program
5. Enable the permissionless thaw operation
4. The Token ACL program will call this gate program during thaw operations

## Development

### Prerequisites
- Rust 1.70+
- Solana CLI 2.2.0+
- Node.js 20.18.0+
- pnpm 9.1.0+

### Building
```bash
# Clone the repository
git clone https://github.com/solana-foundation/token-acl-gate.git
cd token-acl-gate

# Build the program
cargo build-sbf --manifest-path=program/Cargo.toml

# Build CLI
cargo build --manifest-path=cli/Cargo.toml

# Install CLI
cargo install --path cli

# Generate SDKs
pnpm run generate-sdks

# Run tests
cargo test-sbf --manifest-path=sdk/rust/Cargo.toml
```

### Program ID
```
GATEzzqxhJnsWF6vHRsgtixxSB8PaQdcqGEVTEHWiULz
```

### CLI Basic Usage

The CLI provides commands to manage allow/block lists and configure them for token mints.

#### Installing the CLI

```bash
# Install the CLI from crates.io
cargo install token-acl-gate-cli
```

#### Global Options

- `-C, --config <PATH>` - Configuration file to use
- `-k, --payer <KEYPAIR>` - Filepath or URL to a keypair [default: client keypair]
- `-v, --verbose` - Show additional information
- `-u, --url <URL>` - JSON RPC URL for the cluster [default: value from configuration file]

#### Commands

**Create a new list:**
```bash
# Create an allow list
cargo run --bin token-acl-gate-cli -- create-list --mode allow

# Create a block list
cargo run --bin token-acl-gate-cli -- create-list --mode block

# Create an allow-all-eoas list
cargo run --bin token-acl-gate-cli -- create-list --mode allow-all-eoas
```

**Delete a list:**
```bash
cargo run --bin token-acl-gate-cli -- delete-list <LIST_ADDRESS>
```

**Add a wallet to a list:**
```bash
cargo run --bin token-acl-gate-cli -- add-wallet <LIST_ADDRESS> <WALLET_ADDRESS>
```

**Remove a wallet from a list:**
```bash
cargo run --bin token-acl-gate-cli -- remove-wallet <LIST_ADDRESS> <WALLET_ADDRESS>
```

**Apply lists to a mint:**
```bash
# Apply a single list to a mint
cargo run --bin token-acl-gate-cli -- apply-lists-to-mint <MINT_ADDRESS> <LIST_ADDRESS>

# Apply multiple lists to a mint
cargo run --bin token-acl-gate-cli -- apply-lists-to-mint <MINT_ADDRESS> <LIST_ADDRESS_1> <LIST_ADDRESS_2> <LIST_ADDRESS_3>
```

**Example workflow:**
```bash
# 1. Create an allow list
cargo run --bin token-acl-gate-cli -- create-list --mode allow
# Output: list_config: <LIST_ADDRESS>, seed: <SEED>

# 2. Add wallets to the list
cargo run --bin token-acl-gate-cli -- add-wallet <LIST_ADDRESS> <WALLET_ADDRESS_1>
cargo run --bin token-acl-gate-cli -- add-wallet <LIST_ADDRESS> <WALLET_ADDRESS_2>

# 3. Configure the list for a token mint
cargo run --bin token-acl-gate-cli -- apply-lists-to-mint <MINT_ADDRESS> <LIST_ADDRESS>
```


## References

- [Token ACL Repository](https://github.com/solana-foundation/token-acl)
- [sRFC 37 Specification](https://github.com/solana-foundation/SRFCs/discussions/2)
- [Pinocchio Framework](https://github.com/solana-labs/pinocchio)
- [Codama Code Generation](https://github.com/codama-ai/codama)

## License

MIT License - see LICENSE file for details
