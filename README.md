# Freedom Chain

Freedom Chain is the proprietary L1 for the CU Freedom economy.

## M0 — Genesis / Localnet foundation

- Genesis FDM supply: 0
- No premine, founder allocation, investor allocation, or registration airdrop
- Treasury cannot mint FDM
- First protocol-issued FDM can only appear through verified protocol contribution work

## M1 — Accounts / signed transfers

- account public keys and canonical addresses
- Ed25519 transaction signatures
- sequential nonces and replay protection
- integer-only u128 FDM accounting
- signed Transfer transactions
- transaction fees credited to protocol treasury
- state invariants tying supply to balances

## M1.1 — Protocol hardening

- stable transaction type IDs
- domain-separated transaction signing
- signature-inclusive transaction hashes
- invalid signature, nonce, balance and overflow tests

## M2 — Contribution / Work Units

M2 establishes the evidence layer for useful network work without minting FDM yet.

- Freedom Nodes can be registered by signed transactions
- Contributions reference a node and task
- Work Units (WU) measure verified useful work
- quality scores use fixed-point integer values 0..=10_000
- contribution IDs prevent duplicate submission
- contributions move Submitted -> Verified
- verification requires a non-empty set of registered validators
- verified WU is accumulated on the node
- no reward or FDM mint occurs in M2

## M3 — Protocol-issued FDM rewards

- FDM is minted only from verified contribution WU
- reward is calculated with integer fixed-point quality and verification factors
- each contribution can be rewarded exactly once
- epoch issuance has a hard cap; unused capacity does not roll over
- reward mint increases total and circulating supply together
- Treasury and CU have no direct mint path

## M4 — CU Wallet / FDM Billing

- signed Purchase transactions charge paid CU functionality only in FDM
- first catalog product: `AiBotConnect`
- purchase price is denominated in mFDM
- payment moves FDM from buyer to merchant; it does not mint new FDM
- purchase reference prevents duplicate charging
- purchase records provide an on-chain billing receipt
- wallet balance remains authoritative on Freedom Chain

## M5 — Freedom Wallet foundation

- wallet abstraction uses the canonical Freedom address format
- wallet identity is derived from the address and is not a second account type
- FDM display uses integer mFDM with `100_000 mFDM = 1 FDM`
- wallet layer does not mint, burn, or alter balances
- chain state remains the authoritative source of wallet balances

## M6 — Wallet read API / transaction history

- wallet snapshot reads the authoritative chain balance
- balance is exposed in mFDM and human-readable FDM form
- successful transactions are recorded in the chain ledger
- wallet history is derived from chain ledger entries, not a separate balance database
- no wallet API can mint or modify FDM

## Roadmap

- M3: protocol-issued FDM rewards from verified contribution WU
- M4: CU Wallet + Billing + first FDM-only CU feature
