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

## M7 — Billing Engine

- catalog price is defined in protocol state and denominated only in mFDM
- billing supports quote, reserve, capture and refund lifecycle
- reserve moves FDM from available balance to locked balance
- capture transfers reserved FDM to the merchant
- refund returns reserved or captured FDM to the buyer
- billing IDs prevent duplicate reservation
- billing cannot mint FDM

## Roadmap

- M3: protocol-issued FDM rewards from verified contribution WU
- M4: CU Wallet + Billing + first FDM-only CU feature


## M8 — FDM Tickets / Draw Engine

- 1 Ticket = 0.1 FDM = 10,000 mFDM
- tickets are draw-specific and non-transferable
- ticket purchase consumes FDM and never mints FDM
- draw types: daily, weekly, monthly, quarterly, annual, special
- winner count is configurable per draw
- each ticket payment is allocated 60% prize pool, 30% CU Fund, 10% Annual Reserve
- allocation is proportional to actual tickets sold; there is no fixed 100,000-ticket assumption
- draw randomness is derived from finalized chain state and draw-specific seed data
- winner selection is deterministic and independently verifiable from the protocol seed
- prize payouts move existing FDM from protocol treasury to winners; no new FDM is minted
- Ticket allocation is not a burn; true FDM burn remains a separate protocol action


## M9 — Protocol Events / Draw Proof

- transaction receipts expose protocol events
- Ticket creation emits ticket and fund-allocation events
- draw execution emits a deterministic randomness seed and selection proof
- prize payouts emit per-winner events
- the selection proof is `BLAKE3("FREEDOM-DRAW-PROOF-V1" || randomness_seed || ordered_winner_ticket_ids)`
- Treasury allocation remains explicit: Prize Pool, CU Fund, Annual Reserve
- draw payouts reduce the protocol-held Prize Pool and do not mint FDM


## M11 — Transactional block execution / commitments

- Blocks carry ordered receipts alongside ordered transactions.
- Transaction root commits to ordered transaction hashes.
- Receipts root commits to canonical receipt encodings and protocol events.
- Block execution validates height and previous block hash before execution.
- Every transaction is signature-checked and executed before state commitment.
- State root is calculated after block execution and height advancement.
- The committed block hash becomes the next state's last block hash.
- last_block_hash is excluded from state_root to avoid a circular state-root/block-hash dependency.
- A block is accepted only when transaction, receipt and state commitments match.
