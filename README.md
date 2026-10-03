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


## M12 — Deterministic localnet block production

- `freedom-node` produces a deterministic local chain of blocks.
- Three genesis validators are loaded and proposer selection is round-robin over sorted validator IDs.
- Empty blocks exercise TX root, receipts root, state root and block hash commitments.
- Block execution is transactional: failed execution or commitment validation does not partially mutate live state.
- M12 is a local deterministic block-production harness, not yet a networked BFT consensus implementation.


## M13 — Mempool / Transaction Pool / Block Builder

- signed transactions can enter a deterministic mempool
- stale nonces, invalid signatures, wrong chain IDs and duplicate transaction hashes are rejected
- block candidates are ordered deterministically by sender, nonce and transaction hash
- the block builder simulates execution before producing transaction and receipt commitments
- committed transactions are removed from the mempool
- M13 still uses the local deterministic proposer from M12; network propagation and real BFT voting remain future work


## M14 — Signed validator quorum

- validators sign votes over the canonical FREEDOM-VOTE-V1 domain.
- votes are bound to block hash, height and validator ID.
- unknown validators, invalid signatures, wrong block hashes/heights and duplicate votes are rejected.
- quorum uses floor(2N/3) + 1; with 3 validators this requires all 3 votes.
- block finalization checks quorum before executing and committing the block.
- M14 is still a local quorum simulation; network transport and timeout/view-change logic are future work.


## M15 — libp2p validator networking

- freedom-network adds an authenticated libp2p transport using TCP + Noise + Yamux.
- Gossipsub carries separate proposal and validator-vote topics.
- Identify and Ping provide peer identity and liveness signals.
- Network messages serialize BlockProposal and signed ValidatorVote.
- The node binary now starts a real libp2p listener and exposes its PeerId.
- Consensus finalization remains governed by M14 quorum validation; M15 provides transport, not yet automatic peer discovery or view-change logic.


## M16 — Automatic validator protocol loop

- block proposals now carry the full canonical block and a consensus-key signature.
- validators verify proposer identity, proposal signature, transaction signatures, transaction root, receipts, receipts root and post-execution state root before voting.
- validators automatically create signed votes after accepting a proposal.
- votes are collected and checked against the M14 quorum rule.
- once quorum is reached, the block is finalized and committed atomically.
- the validator round resets and the next deterministic proposer can start the next height.
- FREEDOM_VALIDATOR_INDEX=0..2 selects the local validator role.
- FREEDOM_PEERS can provide comma-separated libp2p multiaddrs for peer dialing.
- M16 is the protocol loop; production still needs persistent peer discovery, timeouts/view-change, durable storage and Byzantine fault handling beyond the local quorum path.


## M17 — Peer discovery / timeout / view-change

- libp2p Kademlia is enabled alongside Identify, Ping and Gossipsub.
- Identify listen addresses are fed into Kademlia so discovered peers can enter the routing table.
- FREEDOM_PEERS remains a bootstrap mechanism; Kademlia then maintains discovery beyond the initial peers.
- Block proposals and validator votes are bound to an explicit consensus round.
- proposer selection is deterministic for `(height, round)`.
- validators emit signed timeout messages when a round exceeds the local timeout.
- a `2N/3 + 1` timeout certificate advances the round and changes the deterministic proposer.
- stale proposals, votes and timeout messages from other rounds are ignored/rejected.
- a successful block finalization resets the round and advances to the next height.
- M17 is still a prototype consensus runtime: durable peer state, block sync, persistent chain storage, lockstep timers, equivocation evidence/slashing and full Byzantine view-change rules remain before production.


## M18 — Persistent chain storage / block sync

- freedom-storage persists finalized blocks and chain metadata in RocksDB.
- The validator replays stored blocks through the normal transactional execute_block path on startup.
- Stored blocks are checked for height continuity, previous hash, deterministic proposer, state transition and latest-hash metadata before being accepted.
- Finalized blocks are persisted before the node advances to the next height.
- Validators periodically broadcast bounded sync requests for missing heights.
- Peers answer with at most 32 stored blocks per sync response.
- Sync responses are accepted only for the active request and are replayed sequentially through the same state-transition validation used by consensus.
- Synced blocks are persisted locally after successful execution.
- Block headers now commit the consensus round independently from the block epoch.
- FREEDOM_DB_PATH selects the local RocksDB directory; default is ./data/freedom.
- M18 is a verified-chain replay/sync prototype; production still needs a dedicated request/response transport, state snapshots, block availability scoring, pruning/checkpoints and robust peer-level anti-spam controls.

## M19 — State Snapshots + Fast Startup Checkpoints

Implemented:
- `ChainState` and all state components are serde-serializable for deterministic persistence.
- `freedom-storage` stores validated state snapshots in RocksDB alongside blocks.
- Each snapshot commits `height`, finalized `block_hash`, and deterministic `state_root`.
- Snapshot loading rejects height/state-root mismatches.
- Validator startup restores the latest snapshot, verifies it against the corresponding stored block, then replays only the tail blocks.
- Genesis state is checkpointed automatically when no snapshot exists.
- Finalized and synced blocks create snapshots at `FREEDOM_SNAPSHOT_INTERVAL` (default `1000`).
- `FREEDOM_DB_PATH` remains configurable.

Current limitation: snapshot transfer between peers is not yet implemented. Network sync is still block-based; dedicated request/response snapshot transport and fast-state activation are planned for the next networking milestone.

## M20 — Dedicated Request/Response Sync

Implemented:
- Added a dedicated `/freedom/1/sync/1` libp2p request/response protocol using the maintained JSON codec.
- Validator block-sync requests now target a specific connected peer instead of relying only on Gossipsub broadcast.
- Direct sync responses are replayed through the same proposer, transition, execution and persistence checks as normal chain data.
- Snapshot request/response messages are available over the same direct transport; snapshot activation remains gated until a finalized checkpoint/finality proof is attached.
- Kept the existing Gossipsub sync path as a compatibility fallback during the migration.
- Fixed proposal construction so `BlockProposal.round` is explicitly committed to the proposal metadata.

Reference: libp2p 0.57 exposes the request-response protocol and a built-in JSON codec for serde-compatible request/response types. 

## M21 — Finality-Proof Snapshot Fast Sync

- Snapshot sync now transports the checkpoint block, state root, and a FinalityCertificate containing the validator votes that finalized the checkpoint.
- Validators persist the certificate with protocol-created snapshots.
- A node accepts a remote snapshot only after validating the checkpoint hash/height/state-root commitments, the finality certificate quorum, and the serialized state commitment.
- Accepted snapshots replace local state and persist the checkpoint, so startup/replay can continue from the trusted checkpoint and only replay the tail.
- Snapshots without a finality certificate are not served as fast-sync checkpoints.
- Legacy Gossipsub block sync remains a compatibility path; it is not treated as a finality certificate.
- This is still prototype BFT: validator-set changes, lock/precommit phases, equivocation evidence/slashing, and stronger checkpoint selection remain future work.


## M22 — BFT Finality Core

- Consensus now has explicit PREVOTE and PRECOMMIT phases.
- A validator records a lock after a quorum of matching prevotes and refuses a conflicting proposal while that lock is active.
- PRECOMMIT signatures use a distinct `FREEDOM-PRECOMMIT-V1` signing domain.
- Finalization requires a quorum of matching precommits, not the old single-phase vote quorum.
- Conflicting prevotes/precommits from the same validator are rejected as equivocation evidence.
- `FinalityCertificate` now carries the precommit quorum used for finality; snapshots use this certificate.
- The round-change path is still prototype-level: a full Tendermint-style unlock/valid-round proof and slashing/evidence state are future work.
