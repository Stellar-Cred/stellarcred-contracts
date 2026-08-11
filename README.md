# StellarCred Contracts

**On-chain behavioral reputation and credential protocol on Stellar Soroban**

![Rust](https://img.shields.io/badge/Rust-1.84%2B-orange?logo=rust)
![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-22.0.0-7C3AED)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Stellar Network](https://img.shields.io/badge/Stellar-Network-08B5E5?logo=stellar)
![Drips Wave](https://img.shields.io/badge/Drips-Wave-F59E0B)

## What is StellarCred

StellarCred is the first on-chain behavioral reputation and credential
protocol on Stellar Soroban. Any registered Stellar protocol can issue
**soul-bound (non-transferable) credentials** to a wallet address based on
verified on-chain activity — payments, completed streams, contributions,
wallet age, and manual attestations.

Credentials aggregate into a **reputation score from 0 to 1000** that any
Stellar dApp can query directly on-chain to gate access, adjust terms, or
verify a user's history — with no KYC, no government ID, and no off-chain
infrastructure required.

## How it works

1. A protocol registers as an authorized issuer by calling `register_issuer()`.
2. When a user completes a verifiable on-chain action, the issuing protocol
   calls `issue_credential()` to mint a soul-bound credential to that wallet.
3. Credentials are stored on-chain permanently and can never be transferred
   — there is no transfer entrypoint anywhere in the contract.
4. The contract derives a 0-1000 reputation score from a wallet's credential
   collection using configurable per-type weights.
5. Any Stellar dApp calls `get_score()` or `has_credential()` to read a
   wallet's reputation with a single contract call.

## Built-in Credential Types

| Type | Description | Points |
|---|---|---|
| `PaymentRecord` | Wallet completed a verified on-chain payment | 10 |
| `StreamCompleted` | Wallet fully completed a payment stream | 20 |
| `InvoiceCreator` | Wallet created and settled an invoice | 15 |
| `WillOwner` | Wallet holds a configured on-chain will/inheritance plan | 25 |
| `DeveloperContrib` | Wallet merged a verified open-source contribution | 30 |
| `LongTermHolder` | Wallet has sustained on-chain activity over time | 50 |
| `Verified` | Wallet passed a higher-trust manual/protocol attestation | 100 |

Admins can override any of these weights, or introduce new credential types
entirely, with `set_credential_weight()`.

## Tech Stack

- Rust 1.84+
- [soroban-sdk](https://crates.io/crates/soroban-sdk) 22.0.0
- Stellar CLI (`stellar contract`) for build/deploy tooling

## Local Setup

```bash
git clone git@github.com:Stellar-Cred/stellarcred-contracts.git
cd stellarcred-contracts
rustup target add wasm32-unknown-unknown

# Run the unit test suite
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Build the optimized wasm binary
cargo build --target wasm32-unknown-unknown --release -p cred
```

## Contract Functions

| Function | Description | Parameters | Returns |
|---|---|---|---|
| `register_issuer` | Registers a new authorized credential issuer (admin-only) | `admin, issuer, name, credential_types` | `Result<(), Error>` |
| `remove_issuer` | Deactivates an issuer (admin-only) | `admin, issuer` | `Result<(), Error>` |
| `issue_credential` | Mints a soul-bound credential to a wallet | `issuer, recipient, credential_type, metadata` | `Result<(), Error>` |
| `revoke_credential` | Revokes a credential (original issuer only) | `issuer, recipient, credential_type` | `Result<(), Error>` |
| `get_credentials` | Returns all credentials held by a wallet | `address` | `Vec<Credential>` |
| `get_score` | Computes a wallet's 0-1000 reputation score | `address` | `u32` |
| `has_credential` | Checks whether a wallet holds a given credential type | `address, credential_type` | `bool` |
| `get_issuers` | Lists every registered issuer | — | `Vec<Issuer>` |
| `get_issuer` | Returns a single issuer's record | `issuer` | `Result<Issuer, Error>` |
| `set_credential_weight` | Sets the point value of a credential type (admin-only) | `admin, credential_type, weight` | `Result<(), Error>` |
| `get_credential_weight` | Returns the point value of a credential type | `credential_type` | `u32` |
| `get_leaderboard` | Returns the top N wallets by score | `limit` | `Vec<LeaderboardEntry>` |
| `get_credential_stats` | Returns aggregate issuance counts, total and per type | — | `CredentialStats` |

## Testnet Deployment

The `deploy-testnet` GitHub Actions workflow builds the contract, deploys it
to Stellar testnet via the Stellar CLI, and records the resulting contract
ID in [`deployments/testnet.json`](deployments/testnet.json).

To deploy manually:

```bash
stellar contract build
stellar keys generate deployer --network testnet --fund
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/cred.wasm \
  --source deployer \
  --network testnet \
  -- --admin $(stellar keys address deployer)
```

## Contributing via Drips Wave

This repo participates in the [Drips Stellar Wave](https://www.drips.network/wave/stellar).
Open issues are tagged `complexity: trivial`, `complexity: medium`, or
`complexity: high` with a Point value attached. See
[CONTRIBUTING.md](CONTRIBUTING.md) before picking up an issue — in
particular, **do not start work until you've been assigned**.
