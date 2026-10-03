# Freedom Chain

Freedom Chain is the proprietary L1 for the CU Freedom economy.

## M0 — Genesis / Localnet foundation

M0 establishes the non-negotiable monetary invariant:

- Genesis FDM supply: **0**
- No premine
- No founder allocation
- No investor allocation
- No registration airdrop
- Treasury cannot mint FDM
- First protocol-issued FDM can only appear through verified protocol contribution work

Current M0 implementation:

- Rust workspace
- primitives and canonical address hashing
- Ed25519 signing primitives
- chain state
- genesis validation
- minimum 3-validator genesis
- zero-supply invariant
- basic node executable

## Roadmap

- M1: accounts, signatures, nonces, transfers
- M2: contribution registration, verification and WU
- M3: first protocol-issued FDM
- M4: CU Wallet + Billing + first FDM-only CU feature
