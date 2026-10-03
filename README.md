# Freedom Chain

Freedom Chain is the proprietary L1 for the CU Freedom economy.

## M0 — Genesis / Localnet foundation

- Genesis FDM supply: 0
- No premine, founder allocation, investor allocation, or registration airdrop
- Treasury cannot mint FDM
- First protocol-issued FDM can only appear through verified protocol contribution work

## M1 — Accounts / signed transfers

M1 introduces:

- account public keys and canonical addresses
- Ed25519 transaction signatures
- sequential nonces and replay protection
- integer-only u128 FDM accounting
- signed Transfer transactions
- transaction fees credited to protocol treasury
- state invariants tying supply to balances
- executable unit test proving a signed transfer succeeds once and replay fails

## Roadmap

- M2: contribution registration, verification and WU
- M3: first protocol-issued FDM
- M4: CU Wallet + Billing + first FDM-only CU feature
