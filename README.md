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


## M23 — View Change and Locked-Round Proofs

- Added a signed `TimeoutCertificate` for a quorum of validators that timed out at the previous round.
- Proposals in a later round may carry a `valid_round` plus a precommit quorum proving the locked block from an earlier round.
- Locked blocks are retained across round changes; a proposer must carry the corresponding proof instead of silently replacing a locked block.
- Proposal validation checks timeout certificates and valid-round precommit proofs before accepting a cross-round locked proposal.
- The node now packages timeout quorum evidence during view change and re-proposes a locked block when it has one.
- This remains a prototype BFT implementation; durable evidence/slashing, validator-set changes, and a fully specified unlock/valid-round protocol remain future work.


## M24 — Equivocation Evidence and Slashing Foundation

- Added canonical evidence objects for double-prevote, double-precommit and double-proposal violations.
- Evidence has a deterministic hash and domain-separated signing bytes.
- Consensus can detect and validate equivocation evidence.
- Chain state persists evidence and idempotent slashing records and includes them in the state root.
- Slashing is currently a foundation: validator stake is reduced by an explicit protocol call; automatic evidence transaction wiring and validator-set effects remain future work.


## M25 — Evidence Transactions and Deterministic Slashing

- Added SubmitEvidence transaction type and canonical payload encoding.
- Evidence transactions are executed deterministically by every node.
- Valid evidence is persisted, slashing is idempotent, and validator stake is reduced by the transaction's explicit slash amount capped at current stake.
- Slashing emits EquivocationSlashed and the event is committed into the state root.
- This is the first protocol-level slashing path; stake-weighted quorum and validator-set activation remain M26/M27.


## M26 — Stake-Weighted PoS Quorum

- Added stake-weighted quorum accounting alongside the existing validator-count safety check.
- Votes, precommits and timeout certificates now require at least two-thirds of active validator stake; zero total stake cannot form quorum.
- Added Stake and Unstake payloads and activated validator-owner stake transitions in state execution.
- Stake weight is now available to the consensus core; full validator-set lifecycle and epoch activation remain M27.


## M27 — Epoch-Based Validator Set Lifecycle

- Added an epoch-scoped active validator set snapshot.
- Consensus proposer selection and validator signature validation now use the active epoch set.
- Stake changes do not retroactively change the quorum of an already active epoch.
- Validator set refresh happens only at deterministic epoch boundaries; zero-stake Genesis uses a bootstrap validator set until economic stake is activated.
- Block execution validates the header epoch and commits epoch/set data into the state root.
- Full validator onboarding/exit governance and epoch activation transactions remain future hardening work.


## M28 — Consensus & Network Hardening

- Proposal validation now requires the proposal round to match the block header round.
- Valid-round proofs are validated independently for every proposal.
- Proposal signer lookup uses the active epoch validator set.
- Runtime prevote, precommit and timeout quorum checks use stake-weighted active-set quorum.
- Conflicting timeout messages from the same validator are rejected as equivocation instead of being overwritten.
- Timeout certificates remain composed of individually signed timeout messages and are validated against the earlier round.
- Full peer-level evidence gossip, persistent anti-replay envelopes, and production-grade transport rate limiting remain future hardening work.


## M29 — Consensus Message Anti-Replay

- Added canonical message IDs for proposals, prevotes, precommits and timeouts.
- Added bounded consensus-message deduplication at validator runtime ingress.
- Replayed messages are rejected before entering the consensus state machine.
- Deduplication is bounded to avoid unbounded memory growth.
- Added a runtime test covering duplicate rejection and bounded eviction.
- Message IDs are derived from the existing signed canonical message bytes; no second signing format was introduced.


## M30 — Equivocation Evidence Propagation

- Added signed EvidenceMessage with a dedicated evidence signing domain and message ID.
- Added freedom/1/evidence gossip transport.
- Peers validate evidence sender identity against the active validator set before recording it.
- Evidence messages use the existing bounded anti-replay layer.
- Network round-trip coverage was added for evidence messages.
- Evidence receipt records evidence in state; monetary slashing remains exclusively behind the existing SubmitEvidence transaction path.
- Automatic publication from every locally detected equivocation remains a follow-up hardening step.


## M31 — Automatic Equivocation Evidence Broadcast

- Local double-proposal, double-prevote and double-precommit conflicts now generate deterministic equivocation evidence.
- The observing validator signs an EvidenceMessage and broadcasts it through the dedicated evidence topic.
- Evidence is recorded locally before broadcast and remains subject to the existing anti-replay and peer validation path.
- Timeout conflicts are intentionally not mapped to a slashable evidence kind because Timeout is not currently an EquivocationKind.


## M32 — Deterministic Evidence Slashing

- SubmitEvidence now accepts only the protocol-defined slash amount for the evidence kind and current validator stake.
- DoubleProposal slashing is 5% of current validator stake; DoublePrevote and DoublePrecommit are 10%.
- Slashing is capped by the validator's current stake and is idempotent per evidence hash.
- The validator's owner account staked balance is reduced together with validator stake.
- Slashing remains a deterministic state transition triggered by SubmitEvidence; merely receiving gossip evidence does not slash funds.


## M33 — Validator Exit / Unbonding

- Unstake no longer returns FDM immediately.
- requested stake moves from active validator stake into unbonding_stake.
- unbonding entries are committed in state and have deterministic IDs and release heights.
- default unbonding cooldown is 200 blocks.
- at maturity, unbonded FDM moves from account staked balance back to account balance.
- active validator weight falls immediately, while the validator remains slashable through unbonding stake during cooldown.
- deterministic equivocation slashing covers both active and unbonding stake.
- validator-set refresh uses active stake, so a full validator exit is removed at the next epoch boundary.
- legacy serialized validators without unbonding_stake deserialize as zero via Serde defaults.
- M33 tests cover cooldown, maturity release and slashing of pending unbonding stake.


## M34 — Validator Lifecycle

- Validator lifecycle is explicit: `Active`, `Unbonding`, `Exited`.
- Lifecycle is derived from persistent validator stake/unbonding state; it is not an independently mutable flag.
- Genesis validators are bootstrapped as active until staking is first activated.
- A validator with pending unbonding cannot add new stake until the pending exit matures.
- An exited validator cannot silently resurrect through the genesis bootstrap path.
- Once staking has been activated, epoch refresh includes only validators with positive active stake.
- Full exits therefore produce an empty active set rather than resurrecting the original genesis validator set.
- Zero-amount staking is rejected.
- `staking_activated` is persisted and included in the state commitment.
- M34 tests cover bootstrap, unbonding lockout, exit persistence and pending-unbonding restake rejection.


## M35 — Epoch-Bound Validator Set Changes

- `active_validator_set` is the consensus set for the current epoch and is not mutated by ordinary stake/unstake transactions.
- `pending_validator_set` records the validator set that will become active at the next epoch boundary.
- Stake, unstake, slashing and unbonding maturity update pending membership without changing the current consensus set mid-epoch.
- At an epoch boundary, pending becomes active and a new pending set is derived from the resulting validator state.
- Both active and pending validator sets are committed into the state root.
- Consensus continues to validate proposals, votes, precommits and timeouts against the active set of the current epoch.
- M35 tests cover delayed exit and delayed activation across an epoch boundary.
