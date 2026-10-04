# 321MORE Network

321MORE Network is the proprietary L1 for the CU Freedom economy.

## Network Identity

- Network: **321MORE Network**
- Native Asset: **321M**
- Gas Asset: **321M**
- Ticker / Symbol: **321M**
- Chain ID: **1** (unchanged)
- Native accounting scale: **100,000 native units = 1 321M** (unchanged)
- Consensus, cryptographic domains, transaction wire encoding, reward/emission rules and cross-chain proof semantics are unchanged by this rename.

## M0 — Genesis / Localnet foundation

- Genesis 321M supply: 0
- No premine, founder allocation, investor allocation, or registration airdrop
- Treasury cannot mint 321M
- First protocol-issued 321M can only appear through verified protocol contribution work

## M1 — Accounts / signed transfers

- account public keys and canonical addresses
- Ed25519 transaction signatures
- sequential nonces and replay protection
- integer-only u128 321M accounting
- signed Transfer transactions
- transaction fees credited to protocol treasury
- state invariants tying supply to balances

## M1.1 — Protocol hardening

- stable transaction type IDs
- domain-separated transaction signing
- signature-inclusive transaction hashes
- invalid signature, nonce, balance and overflow tests

## M2 — Contribution / Work Units

M2 establishes the evidence layer for useful network work without minting 321M yet.

- 321MORE Nodes can be registered by signed transactions
- Contributions reference a node and task
- Work Units (WU) measure verified useful work
- quality scores use fixed-point integer values 0..=10_000
- contribution IDs prevent duplicate submission
- contributions move Submitted -> Verified
- verification requires a non-empty set of registered validators
- verified WU is accumulated on the node
- no reward or 321M mint occurs in M2

## M3 — Protocol-issued 321M rewards

- 321M is minted only from verified contribution WU
- reward is calculated with integer fixed-point quality and verification factors
- each contribution can be rewarded exactly once
- epoch issuance has a hard cap; unused capacity does not roll over
- reward mint increases total and circulating supply together
- Treasury and CU have no direct mint path

## M4 — CU Wallet / 321M Billing

- signed Purchase transactions charge paid CU functionality only in 321M
- first catalog product: `AiBotConnect`
- purchase price is denominated in native units
- payment moves 321M from buyer to merchant; it does not mint new 321M
- purchase reference prevents duplicate charging
- purchase records provide an on-chain billing receipt
- wallet balance remains authoritative on 321MORE Network

## M5 — 321MORE Wallet foundation

- wallet abstraction uses the canonical 321MORE address format
- wallet identity is derived from the address and is not a second account type
- 321M display uses integer native units with `100_000 native units = 1 321M`
- wallet layer does not mint, burn, or alter balances
- chain state remains the authoritative source of wallet balances

## M6 — Wallet read API / transaction history

- wallet snapshot reads the authoritative chain balance
- balance is exposed in native units and human-readable 321M form
- successful transactions are recorded in the chain ledger
- wallet history is derived from chain ledger entries, not a separate balance database
- no wallet API can mint or modify 321M

## M7 — Billing Engine

- catalog price is defined in protocol state and denominated only in native units
- billing supports quote, reserve, capture and refund lifecycle
- reserve moves 321M from available balance to locked balance
- capture transfers reserved 321M to the merchant
- refund returns reserved or captured 321M to the buyer
- billing IDs prevent duplicate reservation
- billing cannot mint 321M

## Roadmap

- M3: protocol-issued 321M rewards from verified contribution WU
- M4: CU Wallet + Billing + first 321M-only CU feature


## M8 — 321M Tickets / Draw Engine

- 1 Ticket = 0.1 321M = 10,000 native units
- tickets are draw-specific and non-transferable
- ticket purchase consumes 321M and never mints 321M
- draw types: daily, weekly, monthly, quarterly, annual, special
- winner count is configurable per draw
- each ticket payment is allocated 60% prize pool, 30% CU Fund, 10% Annual Reserve
- allocation is proportional to actual tickets sold; there is no fixed 100,000-ticket assumption
- draw randomness is derived from finalized chain state and draw-specific seed data
- winner selection is deterministic and independently verifiable from the protocol seed
- prize payouts move existing 321M from protocol treasury to winners; no new 321M is minted
- Ticket allocation is not a burn; true 321M burn remains a separate protocol action


## M9 — Protocol Events / Draw Proof

- transaction receipts expose protocol events
- Ticket creation emits ticket and fund-allocation events
- draw execution emits a deterministic randomness seed and selection proof
- prize payouts emit per-winner events
- the selection proof is `BLAKE3("FREEDOM-DRAW-PROOF-V1" || randomness_seed || ordered_winner_ticket_ids)`
- Treasury allocation remains explicit: Prize Pool, CU Fund, Annual Reserve
- draw payouts reduce the protocol-held Prize Pool and do not mint 321M


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
- NETWORK_321MORE_VALIDATOR_INDEX=0..2 selects the local validator role. The legacy FREEDOM_VALIDATOR_INDEX name remains accepted for compatibility.
- NETWORK_321MORE_PEERS can provide comma-separated libp2p multiaddrs for peer dialing. The legacy FREEDOM_PEERS name remains accepted for compatibility.
- M16 is the protocol loop; production still needs persistent peer discovery, timeouts/view-change, durable storage and Byzantine fault handling beyond the local quorum path.


## M17 — Peer discovery / timeout / view-change

- libp2p Kademlia is enabled alongside Identify, Ping and Gossipsub.
- Identify listen addresses are fed into Kademlia so discovered peers can enter the routing table.
- NETWORK_321MORE_PEERS remains a bootstrap mechanism; the legacy FREEDOM_PEERS name is accepted for compatibility. Kademlia then maintains discovery beyond the initial peers.
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
- NETWORK_321MORE_DB_PATH selects the local RocksDB directory; default is ./data/321more. The legacy FREEDOM_DB_PATH name remains accepted for compatibility.
- M18 is a verified-chain replay/sync prototype; production still needs a dedicated request/response transport, state snapshots, block availability scoring, pruning/checkpoints and robust peer-level anti-spam controls.

## M19 — State Snapshots + Fast Startup Checkpoints

Implemented:
- `ChainState` and all state components are serde-serializable for deterministic persistence.
- `freedom-storage` stores validated state snapshots in RocksDB alongside blocks.
- Each snapshot commits `height`, finalized `block_hash`, and deterministic `state_root`.
- Snapshot loading rejects height/state-root mismatches.
- Validator startup restores the latest snapshot, verifies it against the corresponding stored block, then replays only the tail blocks.
- Genesis state is checkpointed automatically when no snapshot exists.
- Finalized and synced blocks create snapshots at `NETWORK_321MORE_SNAPSHOT_INTERVAL` (default `1000`). The legacy FREEDOM_SNAPSHOT_INTERVAL name is accepted for compatibility.
- `NETWORK_321MORE_DB_PATH` remains configurable; the legacy FREEDOM_DB_PATH name is accepted for compatibility.

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

- Unstake no longer returns 321M immediately.
- requested stake moves from active validator stake into unbonding_stake.
- unbonding entries are committed in state and have deterministic IDs and release heights.
- default unbonding cooldown is 200 blocks.
- at maturity, unbonded 321M moves from account staked balance back to account balance.
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


## M36 — Validator Set Transition Certificate

- Added a canonical `validator_set_hash` committed over validator id, owner, consensus key and active stake.
- Added `ValidatorSetVote` with the dedicated `FREEDOM-VALSET-VOTE-V1` signing domain.
- Added `ValidatorSetCertificate` with the dedicated `FREEDOM-VALSET-CERT-V1` domain.
- Certificate validation binds the transition to the current epoch, the committed active-set hash and the derived pending-set hash.
- Every certificate vote must be signed by a validator in the current active set; duplicate signers and malformed signatures are rejected.
- Certificate acceptance requires stake-weighted quorum from the current active validator set.
- The accepted certificate is committed into `ChainState` and therefore into the state root.
- Runtime exposes `commit_validator_set_certificate()` as the single validation-gated state transition API.
- M36 is the transition-proof foundation. Automatic attachment of the certificate to an epoch-bound block and snapshot fast-sync activation are intentionally left for the next integration milestone.


## M37 — Epoch Transition Proof → Block → Snapshot

- `BlockHeader` now carries an optional `epoch_transition_certificate`.
- The certificate hash is part of canonical block serialization and therefore the block hash.
- Epoch-boundary blocks must carry a valid validator-set transition certificate; non-boundary blocks must not carry one.
- Runtime block execution enforces the epoch-transition proof before committing the block.
- State snapshots explicitly carry the epoch-transition certificate alongside the state and finality certificate.
- Snapshot loading rejects a mismatch between the transported epoch proof and the certificate committed in `ChainState`.
- The snapshot envelope is therefore no longer only a state dump: it carries the validator-set transition proof needed to audit the epoch boundary.
- Automatic network collection/broadcast of validator-set votes and automatic snapshot activation from a remote peer remain the next integration step; this milestone makes the proof part of the authenticated block/snapshot path.


## M38 — Network Epoch Certificate Protocol

- Added dedicated gossip topics for validator-set votes and validator-set certificates.
- Added `NetworkMessage::ValidatorSetVote` and `NetworkMessage::ValidatorSetCertificate`.
- Added signed validator-set vote creation bound to the current active set and next pending set hash.
- Added strict network vote validation: epoch, active-set membership, set hashes, key binding and Ed25519 signature.
- Runtime can collect validated votes and produce a `ValidatorSetCertificate` only after stake-weighted quorum.
- Added wire encoding helpers for publishing votes/certificates through the network layer.
- The protocol now has the authenticated network primitives required for automatic epoch certificate collection; node event-loop wiring and automatic certificate broadcast remain the next integration step.


## M39 — Node Event-Loop Integration

- The node automatically emits a signed `ValidatorSetVote` when the next block is an epoch transition.
- Incoming validator-set votes are deduplicated, cryptographically validated and accumulated by validator id.
- Once stake-weighted quorum is reached, the node constructs and gossips a `ValidatorSetCertificate`.
- Incoming certificates are validated before being committed to the local state used by block production.
- Epoch-bound block construction now passes the correct next epoch instead of a hard-coded epoch value.
- Runtime block execution commits the certificate carried by an epoch-transition block before calculating the resulting state root.
- The node therefore has the full local event-loop path from epoch boundary detection to certificate gossip and certificate-bearing block production.


## M40 — Snapshot Fast-Sync Activation

- Snapshot sync responses now carry the epoch-transition certificate committed by the snapshot state.
- Snapshot storage rejects inconsistent epoch-transition proofs instead of performing a tautological hash check.
- A fresh node requests the latest snapshot from a connected peer.
- Snapshot activation verifies the checkpoint hash/state root, finality certificate against the snapshot validator set, state commitment, and epoch-transition certificate binding.
- The checkpoint block must carry the same epoch-transition certificate as the snapshot state.
- Only after these checks does the node replace its local state and persist the verified snapshot.


## M41 — Snapshot Security & Trust Hardening

- Snapshots are bound to CHAIN_ID in both local storage and the network sync envelope.
- A node rejects a snapshot from another chain before state activation.
- Block transition validation now enforces epoch-transition certificate rules through consensus validation.
- Snapshot activation continues to require checkpoint hash/state-root consistency and a valid finality certificate against the snapshot validator set.
- Epoch-transition certificates are bound to both the snapshot state and its checkpoint block.


## M42 — Snapshot Checkpoint Finality

- Finality certificate quorum validation is stake-weighted for precommits.
- Added checkpoint-bound finality validation: certificate block hash, height and round must exactly match the checkpoint block.
- Snapshot activation uses the checkpoint-bound validator instead of validating a certificate in isolation.
- Stored snapshots with finality certificates revalidate the certificate against their persisted checkpoint block before being returned.


## M42 — Snapshot Checkpoint Finality

- Finality verification is now explicitly bound to the checkpoint block hash, height and round.
- Precommit quorum verification is stake-weighted and can operate against an explicit historical validator set.
- Snapshots retain the validator set that actually signed the checkpoint finality certificate.
- Snapshot storage validates that certificate against the stored checkpoint and historical validator set.
- Snapshot transport carries that historical validator set, and the receiving node verifies its canonical set hash against the known epoch-transition proof or snapshot active set before accepting the certificate.
- At epoch boundaries the finality proof is therefore checked against the previous epoch validator set rather than the already-activated next epoch set.


## M42 — Snapshot Checkpoint Finality

- Non-genesis snapshots are now required to carry a FinalityCertificate.
- Stored snapshots verify that the certificate block hash/height exactly match the snapshot checkpoint.
- The stored checkpoint block hash and state root are independently checked against the snapshot envelope.
- At an epoch transition, the finality validator set must hash to the transition certificate's previous_set_hash; after the transition, the snapshot active set may be the new set.
- Remote snapshot activation applies the same checkpoint/finality binding and rejects genesis as a remote finalized checkpoint.
- This closes the boundary between a valid state snapshot and a merely self-consistent state dump.


## M43 — Sync Transport Hardening

- Block sync requests are bounded to the protocol maximum and cannot start from genesis.
- Block sync responses must contain a non-empty, contiguous sequence of at most MAX_SYNC_BLOCKS blocks.
- Remote snapshot requests cannot target genesis.
- Snapshot state payloads are bounded to 8 MiB before deserialization.
- Snapshot responses are checked for checkpoint height/state-root and finality-certificate identity before deeper cryptographic validation.
- These checks run at the node/network boundary before state activation or block execution.

## M44 — Snapshot Recovery Integrity

- genesis snapshots are treated as a special height-0 state and do not require a stored checkpoint block;
- non-genesis snapshots require a matching stored checkpoint and finality data;
- recovery rejects snapshots that are ahead of the stored chain height;
- recovery verifies the latest block hash against persistent height/hash metadata before replay;
- replay continues to enforce height, previous-hash, proposer, execution, and final-hash continuity;
- corrupted snapshot linkage and missing checkpoint cases have storage regression tests.

## M45 — Crash/Restart Continuation

- local proposal construction is shared between normal startup and post-recovery paths;
- after block sync, a node that becomes the next proposer resumes proposing immediately;
- after snapshot activation, a node resumes proposing from the recovered height when it is the scheduled proposer;
- the next proposal must use recovered height + 1, recovered last_block_hash, and the deterministic proposer for the recovered round;
- regression coverage verifies contiguous proposal construction from recovered genesis state.

## M46 — Adversarial Replay Integrity

- persisted replay is isolated behind one validation path;
- height gaps, previous-hash corruption, and proposer corruption are rejected before state mutation;
- block execution errors abort recovery rather than partially activating the corrupted replay;
- replay still verifies the resulting committed block hash after execution;
- regression tests cover corrupted previous hash, proposer, and height continuity.

## M47 — Storage Atomicity and Tip Integrity

- block bytes, `meta:height`, and `meta:last_hash` are committed through one RocksDB write batch;
- zero-height blocks cannot be persisted as normal chain blocks;
- recovery validates that tip metadata points to an actual persisted block with the matching hash;
- genesis metadata must remain zero-hash when the stored height is zero;
- regression tests cover invalid zero-height writes and corrupted tip hash metadata.


## M48 — Snapshot Atomicity and Consistency

- snapshot persistence validates chain identity, state height/state root, and checkpoint linkage before writing;
- non-genesis snapshots must match the persisted checkpoint block hash, checkpoint state root, and the state's last block hash;
- non-genesis snapshots must pass finality-certificate validation against the persisted checkpoint and historical validator set before persistence;
- genesis snapshots remain a special height-0 case and may retain the bootstrap validator set, but cannot carry finality or epoch-transition certificates;
- epoch-transition certificate bindings are validated before persistence and again during snapshot loading;
- invalid snapshot writes fail before creating a snapshot record, so callers cannot persist a logically inconsistent checkpoint;
- regression tests cover missing checkpoints, checkpoint/state-root mismatch, and snapshot linkage failures.


## M49 — Atomic Remote Snapshot Activation

- remote snapshot activation persists the checkpoint, tip metadata, and validated snapshot through one RocksDB write batch;
- live ChainState is replaced only after that persistence succeeds;
- snapshot validation can run against the incoming checkpoint before either record is committed;
- a failed activation cannot leave the node with an in-memory snapshot that was not durably persisted;
- regression coverage verifies that a checkpoint/state linkage failure leaves both block metadata and snapshot storage unchanged.


## M50 — Atomic Block Sync Persistence

- synced blocks are first validated and executed against a cloned working state;
- the complete accepted sync batch is persisted with one RocksDB write batch;
- live ChainState is replaced only after the complete batch is durably persisted;
- a persistence failure cannot leave a partially persisted sync batch with a corresponding in-memory state transition;
- invalid members are rejected before any block from the batch is written;
- sync no longer attempts to create non-finalized snapshots from an ordinary block-sync response; finalized snapshots remain tied to finality certificates.


## M51 — Sync Batch Boundary Hardening

- atomic block persistence now rejects zero-height and non-contiguous batches before any RocksDB write;
- batch metadata is committed once, from the final block in the accepted contiguous batch;
- the node no longer sends an invalid genesis snapshot request (`height=0`), because snapshot transport explicitly excludes genesis;
- genesis recovery remains local, while non-genesis snapshot recovery uses checkpoint-backed snapshots.


## M52 — Sync Response Trust Binding

- block-sync responses are now bound to both the outstanding request ID and the exact requested starting height;
- stale or replayed responses for a different height are discarded before execution;
- the network boundary explicitly requires the first response block to equal `from_height`;
- the pending sync height is cleared together with the request ID after successful application.


## M53 — Snapshot Checkpoint Commitment Hardening

- snapshot responses now have their checkpoint block hash verified at the network boundary;
- snapshot certificate round must match the checkpoint round before the payload reaches node activation logic;
- the node retains its deeper certificate, validator-set, epoch-transition, state-root and atomic-persistence checks as a second validation layer.


## M54 — Snapshot Trust Anchor

- remote snapshot responses are accepted only when they match an explicitly tracked pending snapshot request ID and target height;
- unsolicited, stale, or replayed snapshot responses are rejected before any certificate or state processing;
- successful snapshot activation clears the pending snapshot request together with normal sync state;
- the node does not treat a cryptographically valid but unsolicited snapshot as a new trust anchor.


## M55 — Remote Snapshot Discovery

- peers can answer a snapshot request with the latest finalized snapshot at or below the requested target height;
- the node requests the latest available snapshot on peer connection using a bounded discovery target;
- the response remains bound to the outstanding snapshot request ID and passes the M53 checkpoint/certificate checks before activation;
- after successful atomic snapshot activation, normal block sync resumes from the recovered height.


## M56 — Transport-Level Sync Trust Binding

- block-sync responses are now bound to the exact libp2p outbound request ID in addition to the protocol request hash and requested height;
- snapshot responses are now bound to the exact outbound request ID, destination peer, protocol request hash and requested target height;
- a response from a different peer cannot satisfy a pending request merely by reproducing the deterministic protocol `request_id`;
- snapshot responses are explicitly rejected when their returned height exceeds the requested target;
- this closes the remaining gap between application-level request correlation and the authenticated request-response transport.


## M57 — Multi-Peer Sync Selection

- periodic block synchronization no longer always trusts the first connected peer;
- sync requests rotate deterministically across the currently connected peers;
- a new sync request is not issued while another outbound sync request is still pending;
- disconnecting a pending sync/snapshot peer clears that stale request anchor;
- peer rotation remains fail-closed because every response is still bound to its exact peer and transport request ID;
- regression coverage verifies deterministic peer rotation and empty-peer handling.


## M58 — Multi-Peer Sync Verification

- periodic sync requests are sent to up to two connected peers for the same starting height;
- returned block batches are fingerprinted and compared before state/persistence changes;
- when two peers are available, a batch is accepted only after two matching responses;
- when only one peer is available, the node retains a single-peer bootstrap path;
- disconnects clear outstanding candidates tied to the lost sync round;
- the old unauthenticated gossipsub sync request/response path is no longer accepted; synchronization uses the authenticated request-response transport with M56 peer/request binding.


## M59 — Peer Disagreement Handling

- sync rounds now record which peers participated and the starting height they were asked to serve;
- when peers return different block batches for the same sync round, the node records the conflicting peer and batch height instead of selecting one arbitrarily;
- disagreement evidence is scoped to the current sync round and is cleared after the round ends or a peer disconnects;
- a disagreement does not by itself label a peer malicious: the node remains neutral until stronger cryptographic or protocol evidence exists;
- the sync quorum still requires matching batches before state or storage changes.


## M60 — Sync Misbehavior Evidence

- sync disagreements now produce structured in-memory evidence containing the peer identity, requested height, protocol request ID, offending batch fingerprint and reason;
- evidence is created only after both responses pass transport and network-level validation, so ordinary malformed traffic is not confused with a valid-but-conflicting batch;
- evidence remains scoped to the active sync round and is cleared when that round ends or its peers disconnect;
- the evidence layer does not automatically punish or label a peer; it provides a deterministic input for a future peer-scoring/isolation layer.


## M61 — Peer Sync Scoring & Cooldown

- repeated valid-but-conflicting sync batches now increment a local peer sync-misbehavior score;
- peers enter a temporary sync cooldown after three recorded conflicts;
- cooldown is height-based and expires after three chain heights, avoiding permanent exclusion from one local observation;
- peer scoring affects sync selection only and does not alter consensus validator status or slash stake;
- a peer that has never produced evidence remains eligible by default.


## M62 — Multi-Peer Snapshot Verification

- snapshot discovery can collect finalized snapshots from up to two eligible peers;
- each response must first pass the existing transport, checkpoint, finality-certificate, validator-set, epoch-transition and state-commitment checks;
- verified snapshot commitments are fingerprinted from height, block hash, state root, certificate, finality validator-set hash and epoch-transition certificate;
- when two snapshot peers participate, activation requires matching commitments from both;
- differing snapshot commitments are recorded as a conflict and never selected arbitrarily;
- snapshot activation continues to use the atomic checkpoint+snapshot persistence path from M49.


## M63 — Snapshot Conflict Evidence & Peer Scoring

- fully validated snapshot responses with different commitments now produce structured snapshot conflict evidence;
- evidence records peer identity, snapshot height, protocol request ID, commitment fingerprint and reason;
- the conflicting peer receives the existing bounded sync-misbehavior score and therefore participates in the same temporary cooldown policy;
- scoring occurs only after transport, checkpoint, finality, validator-set, epoch-transition and state-commitment validation;
- a single snapshot disagreement does not permanently isolate or slash a peer;
- snapshot conflict evidence is scoped to the active snapshot round and cleared after successful activation.


## M64 — Unified Peer Reputation Layer

- sync and snapshot conflicts now feed one PeerReputation model;
- evidence is typed as SyncConflict or SnapshotConflict and keeps peer, height, request ID, fingerprint and reason;
- one score and one cooldown policy determine peer eligibility for sync and snapshot discovery;
- reputation remains local node evidence and does not change validator membership, stake, consensus weight or slashing;
- new evidence kinds can be added without duplicating scoring and cooldown logic.


## M65 — Peer Reputation Persistence

- peer reputation is now persisted in the chain store and restored during node startup;
- the persisted record includes peer scores, cooldown heights and structured evidence;
- updates are written atomically as a single RocksDB key;
- missing reputation data safely starts from an empty reputation map;
- reputation remains local operational evidence and is not part of consensus state or validator stake.


## M66 — Peer Reputation Decay & Recovery

- reputation score now decays by one point per 10 chain heights without new evidence;
- decay runs from the node sync timer and persists the updated reputation;
- historical evidence is retained for audit and is not deleted by decay;
- an already-issued cooldown is never shortened by decay;
- peer eligibility requires both an expired cooldown and a score below the misbehavior threshold;
- sustained clean operation can therefore restore a peer's sync/snapshot eligibility without a permanent blacklist.


## M67 — Reputation Evidence Compaction

- each peer retains at most 32 detailed reputation evidence records;
- adding new evidence evicts the oldest detailed records while preserving score and cooldown state;
- decay also enforces the same bound for previously persisted or legacy reputation data;
- the bounded journal prevents long-running nodes from growing reputation storage without limit;
- evidence compaction does not alter the accumulated score or an already-issued cooldown.


## M68 — Reputation Policy Separation

- peer reputation policy is represented explicitly by threshold, cooldown, decay interval, and evidence retention settings;
- evidence recording remains responsible for recording bounded evidence, while policy application owns score-based cooldown decisions;
- eligibility reads the same policy instead of duplicating threshold constants;
- current policy values remain unchanged: threshold 3, cooldown 3 rounds, decay interval 10 heights, evidence limit 32.


## M69 — Reputation Evidence Classification

- sync and snapshot conflicts are explicitly classified by evidence kind;
- reputation policy assigns a weight to each evidence class;
- current weights remain neutral at 1 for both classes, preserving existing behavior;
- future policy changes can increase or decrease the score impact of a specific evidence class without changing the evidence journal format.


## M70 — Reputation Evidence Severity

- reputation evidence now carries explicit severity: Normal, Elevated, or Critical;
- policy applies a severity multiplier after the evidence-kind weight;
- current production evidence remains Normal, preserving existing scoring behavior;
- policy multipliers are 1x Normal, 2x Elevated, and 3x Critical;
- severity is persisted with evidence, so later policy inspection can distinguish ordinary and elevated incidents.


## M71 — Automatic Reputation Evidence Severity

- severity is derived deterministically from recent evidence context instead of being manually selected;
- first and second recent conflicts of the same evidence kind are Normal;
- the third recent conflict of the same kind is Elevated;
- the fourth and subsequent recent conflicts are Critical;
- the classification is bounded to recent evidence, so compaction remains compatible with the policy.


## M72 — Independent Confirmation Severity

- severity policy now accepts an independent-confirmation count;
- one confirming source remains Normal unless repetition rules escalate it;
- two independent confirmations produce Elevated severity;
- three or more independent confirmations produce Critical severity;
- the current recording path remains conservative and supplies one confirmation until the transport layer provides explicit independent-peer context;
- repeated evidence from the same peer is not treated as independent confirmation.


## M73 — Live Quorum Confirmation

- sync and snapshot conflict evidence now receives the real number of peers supporting the strongest alternative candidate;
- evidence recording uses that quorum count to derive severity;
- repeated evidence from one peer still cannot create independent confirmation;
- sync and snapshot paths now share the same quorum-aware reputation policy;
- the existing one-peer path remains Normal, while a two-peer independent confirmation becomes Elevated and three or more becomes Critical.


## M74 — Reputation Evidence Deduplication

- identical evidence is idempotent and cannot increase reputation score more than once;
- the deduplication identity is `(peer, height, request_id, fingerprint, evidence kind)`;
- repeated processing of the same fact does not append another journal entry;
- distinct fingerprints remain independently countable;
- deduplication happens before severity classification and score mutation.


## M75 — Persistent Reputation Evidence Deduplication

- reputation evidence keeps a persistent compact deduplication index of event IDs;
- the detailed evidence journal remains bounded at 32 records per peer;
- event IDs are domain-separated hashes of peer, height, request ID, fingerprint, and evidence kind;
- repeated processing of an event remains rejected even after its detailed journal record has been compacted out;
- the new field is backward-compatible with persisted reputation data, and startup rebuilds the index from legacy evidence before continuing.


## M76 — Reputation Persistence Migration Safety

- persisted reputation data from before M75 remains deserializable because the dedup index defaults to an empty set;
- startup reconstructs dedup IDs from the retained legacy evidence journal before the node processes network input;
- a regression test verifies that a legacy record cannot be counted again after migration;
- reputation remains local node security state and is not promoted into consensus state merely to solve persistence concerns.


## M77 — Universal Asset Registry

- 321MORE Network now has a consensus-visible asset registry inside ChainState;
- every asset has a cryptographic asset_id derived from source chain, asset type, canonical contract/native identifier, issuer, and bridge;
- symbols are metadata only, so assets such as USDT on different chains remain distinct even when their symbols match;
- 321M is registered as the native 321MORE Network asset at genesis;
- external assets are represented as Bridged assets without assuming an EVM contract model;
- asset identity is validated before registration and duplicate asset IDs are rejected;
- the asset registry is included in the deterministic state root and therefore inherits existing block/snapshot state-root validation;
- the registry field uses serde defaulting for backward-compatible state decoding while new genesis state always contains 321M.


## M78 — Bridge Message Domain Separation

- added canonical BridgeMessage with protocol version, source/destination chain IDs, bridge ID, asset ID, source transaction hash, source event index, nonce, amount, recipient, and message hash;
- message hashing uses a dedicated FREEDOM bridge domain and canonical binary field ordering;
- the cryptographic message hash binds source chain, destination chain, bridge, asset, nonce, amount, and recipient, preventing cross-domain reinterpretation;
- replay identity additionally binds source chain, source transaction, source event/output index, and nonce;
- no bridge message is considered cryptographically valid unless message_hash matches the canonical hash.

## M79 — Persistent Bridge Replay Protection

- processed bridge messages are now persisted directly in consensus-visible ChainState;
- every processed message is indexed by the canonical replay key derived from source chain, source transaction, source event/output index, and nonce;
- an exact replay of an already processed message is rejected as DuplicateBridgeMessage;
- a conflicting payload that reuses an existing replay identity is rejected rather than silently overwriting the original message;
- bridge messages must target 321MORE Network, use the supported protocol version, have a non-zero amount and a valid message hash;
- the referenced asset must exist, be Active, be Bridged, and match both the source chain and bridge ID carried by the message;
- replay records are included in the deterministic state root, so replay protection survives snapshots and consensus state recovery;
- the replay index uses serde defaulting so older serialized ChainState values can deserialize with an empty replay set;
- M79 records verified bridge messages but does not mint or release assets yet; mint/burn accounting is intentionally deferred to the bridge state machine and supply-invariant milestones.

## M80 — Bridge Message State Machine

- bridge messages now have an explicit consensus-visible lifecycle: Verified → Accepted → Minted;
- recording a cryptographically valid bridge message enters the Verified state;
- acceptance is only possible from Verified, and Minted is only reachable from Accepted;
- transitions are forward-only and repeated transitions are rejected;
- bridge lifecycle status is included in the deterministic state root;
- M79 bridge records remain deserializable because lifecycle status is stored in a separate serde-defaulted map;
- legacy M79 records without a status are safely treated as Verified when first accepted, providing an explicit migration path;
- M80 does not yet create supply or mint balances: the Minted state is a state-machine transition only, while actual supply accounting is deferred to the supply-invariant milestone.


## M81 — Universal Bidirectional Bridge Accounting

- bridge withdrawals are now modeled generically for any external network; the protocol does not hard-code Bitcoin, Ethereum, TRON, TON, Solana, or another chain into the withdrawal state machine;
- external recipient addresses are stored as opaque bytes, allowing UTXO addresses, EVM/TRON contract addresses, TON/Solana addresses, and other network-specific formats to be handled by separate adapters;
- outbound lifecycle is consensus-visible and forward-only: Requested → Validated → Approved → Signing → Broadcast → ExternalConfirmed → Finalized, with controlled rejection from the early stages;
- withdrawal identity is bound to bridge, asset, destination chain, sender, and nonce, while the request hash commits to the complete withdrawal payload including the external recipient;
- bidirectional bridge accounting tracks verified external locked, 321M minted, 321M burned, and external released amounts;
- accounting enforces the conservation invariant minted - burned == external_locked - external_released, plus monotonic safety bounds preventing mint above verified lock, burn above minted, or release above burned;
- final external release cannot be finalized until the corresponding 321MORE-side representation has been burned;
- bridge withdrawals, lifecycle status, and accounting are included in the deterministic state root and survive serde-defaulted state migration;
- M81 is the protocol accounting/state-machine foundation only: it does not yet broadcast real Bitcoin/EVM/TRON/TON/Solana transactions or hold private bridge signing keys;
- network-specific adapters will be implemented separately, with Bitcoin using a UTXO model and EVM/TRON/TON/Solana using their respective transaction/finality models.


## M82 — Universal Bridge Adapter Layer

- added a chain-agnostic `BridgeAdapter` interface separating bridge protocol state from external-network mechanics;
- added dedicated adapters for Bitcoin, EVM-compatible networks including Ethereum, TRON, TON, and Solana;
- Bitcoin uses confirmation-based finality, EVM/TRON/TON use block-finality semantics, and Solana uses slot-finality semantics;
- deposit verification binds bridge ID, source chain ID, transaction hash, proof bytes, and required finality depth before a deposit can be trusted by the bridge layer;
- withdrawal intents use a canonical domain-separated hash and opaque recipient bytes, so network-specific address formats do not leak into consensus assumptions;
- adapters are deliberately limited to verification and intent construction at this milestone: no RPC clients, private keys, external transaction broadcasting, or automatic minting are embedded in validator processes;
- the architecture is extensible through additional adapter kinds without changing the bidirectional accounting state machine.


## M83 — Bitcoin UTXO Adapter

- added a Bitcoin-specific UTXO deposit model with transaction hash, vout, satoshi amount, block height, confirmation depth, Merkle branch/index, and scriptPubKey;
- Bitcoin deposit identity uses `tx_hash + vout`, matching the UTXO/output model and feeding the existing bridge replay identity through the event index;
- deposits require the configured bridge ID, Bitcoin chain ID, non-zero amount, scriptPubKey, Merkle proof, and required confirmation depth;
- Bitcoin withdrawals use an opaque recipient script/bytes representation and produce a canonical 321MORE withdrawal intent without embedding private keys or transaction signing in consensus;
- the adapter is currently a verification/intent layer: it does not yet connect to Bitcoin RPC, select UTXOs, construct signed transactions, broadcast transactions, or verify a live Merkle proof cryptographically;
- the same bridge protocol remains reusable for EVM, TRON, TON, Solana, and future adapters.


## M84 — Ethereum / EVM Adapter

- added an EVM transfer proof model for ERC-20-style assets, including EVM chain ID, transaction hash, block number, confirmation depth, token contract, sender, recipient, amount, log index, event signature, and proof bytes;
- the EVM bridge uses `chain_id` to distinguish Ethereum, BNB Chain, Polygon, Arbitrum, Base, and other EVM networks while keeping the adapter implementation shared;
- `tx_hash + log_index` becomes the external event identity used by the existing bridge replay model;
- ERC-20 deposits require the configured bridge, chain, token contract, participants, non-zero amount, event signature, proof bytes, and required block finality;
- EVM withdrawals require a canonical 20-byte recipient and produce a domain-separated withdrawal intent;
- no EVM RPC, contract execution, private-key custody, or transaction broadcasting is performed by consensus code at this milestone; those operations belong in the external bridge executor/signer layer.


## M85 — TRON / TRC-20 Adapter

- added a dedicated TRC-20 transfer proof model with TRON chain ID, transaction hash, block number, confirmation depth, token contract, sender, recipient, amount, event index, event signature, and proof bytes;
- TRC-20 deposit identity maps `tx_hash + event_index` into the existing bridge replay domain;
- deposits require the configured bridge and chain, non-zero transaction/token/participants/amount, event signature, proof bytes, and the adapter's configured block-finality threshold;
- withdrawals require a canonical TRON recipient byte representation and produce the existing domain-separated withdrawal intent;
- this milestone does not perform TRON RPC calls, ABI/event decoding, private-key custody, proof cryptography, or transaction broadcasting; those remain outside consensus state and belong to the bridge executor/signer layer.


## M86 — TON / Jetton Adapter

- added a dedicated TON Jetton transfer proof model with TON chain ID, transaction hash, block/sequence number, confirmation depth, Jetton master, sender, recipient, amount, event index, event signature, and proof bytes;
- Jetton deposits bind `tx_hash + event_index` into the existing bridge replay identity;
- deposits require the configured bridge and chain, non-zero transaction/Jetton/participants/amount, event signature, proof bytes, and the adapter's configured finality threshold;
- withdrawals preserve the canonical TON recipient bytes in a domain-separated withdrawal intent;
- no TON RPC, BOC/TVM proof verification, private-key custody, or transaction broadcasting is performed by consensus code at this milestone; those belong in the external bridge executor/verifier layer.


## M87 — Solana / SPL Token Adapter

- added a Solana transfer proof model for SPL-style assets with Solana chain ID, transaction signature/hash, slot, slot-finality depth, token mint, source/destination accounts, amount, instruction index, program ID, and proof bytes;
- SPL transfer replay identity maps `transaction + instruction_index` into the existing bridge replay domain;
- deposits require the configured bridge and chain, non-zero transaction/token/accounts/amount, program ID, proof bytes, and the adapter's configured slot-finality threshold;
- withdrawals require a canonical 32-byte Solana recipient representation and produce the existing domain-separated withdrawal intent;
- the adapter model is also suitable as the foundation for native SOL once the external verifier distinguishes native transfers from SPL token transfers;
- no Solana RPC, transaction/message decoding, Merkle/account proof verification, private-key custody, or transaction broadcasting is performed by consensus code at this milestone.


## M88 — Bridge Security Policy

- added a chain-agnostic `BridgeSecurityPolicy` shared by all external bridge adapters;
- supports validator quorum, threshold-signature requirements, or both through `BridgeApprovalMode`;
- validator approval requires a strict two-thirds-plus-one style supermajority (`3 * quorum > 2 * total`);
- external finality, per-deposit and per-withdrawal amount limits, daily withdrawal limits, emergency pause, and nonce floor are explicit policy parameters;
- policy helpers reject deposits/withdrawals while paused, over configured limits, or below the nonce floor;
- the policy is a consensus-visible configuration primitive, while actual signer/MPC cryptography and off-chain daily-volume accounting remain executor-layer responsibilities.


## M89 — Bridge Approval Records

- added concrete `BridgeApproval` records binding a validator approval to bridge ID, request ID, withdrawal ID, validator ID, nonce, amount, destination chain, asset, intent hash, and signature bytes;
- approval signing bytes use a dedicated domain and bind the exact withdrawal intent, preventing an approval from being reused for another amount, recipient, asset, chain, nonce, or request;
- `BridgeApprovalSet` deduplicates validators and evaluates quorum against the exact bridge/withdrawal/intent tuple;
- threshold signatures are tracked separately from validator quorum, allowing `Quorum`, `Threshold`, or `QuorumAndThreshold` policy modes;
- malformed or mismatched approvals do not count toward quorum;
- cryptographic signature verification and MPC aggregation remain executor/crypto-layer responsibilities; this milestone defines the consensus-visible approval evidence and counting rules.


## M90 — Bridge Approval Authentication

- bridge approvals can now be authenticated with the chain's existing Ed25519 crypto primitive;
- validator identity is bound as `hash(public_key) == validator_id`, preventing a valid signature from being presented under another validator identity;
- signature verification covers the domain-separated approval hash, which includes bridge, request, withdrawal, validator, nonce, amount, destination chain, asset, and intent hash;
- `BridgeApprovalSet::authenticated_approvals` counts only structurally valid approvals whose registered validator public key matches the claimed identity and whose signature verifies;
- duplicate validators remain deduplicated;
- threshold/MPC aggregate signature verification is intentionally still separate because the concrete external signer scheme may differ by bridge deployment.


## M94 — Atomic Bridge Withdrawal Rate-Limit Approval

Bridge withdrawal rate limits are now reserved atomically with authenticated approval. A withdrawal can enter `Approved` only from `Validated`; bridge period usage is reserved before the status transition, and a failed reservation leaves the withdrawal in `Validated`. Repeated approval calls cannot consume the limit twice. The reserved period is persisted and included in the state root.


## M95 — Authenticated Bridge Emergency Pause

Bridge emergency pause is now a consensus-visible, validator-authenticated action. A `BridgePauseAuthorizationSet` binds bridge ID, pause state and a monotonic nonce; each authorization is Ed25519-authenticated against the validator identity. The configured validator quorum must approve a uniform pause/unpause state. Older or replayed authorization sets are rejected, and the latest authorization set is persisted in the state root.


## M96 — Universal Bridge Nonce Registry

Outbound withdrawals now use a persistent nonce registry keyed by bridge, asset and direction. A nonce can only be reserved once for a given bridge/asset outbound lane, preventing replayed or duplicated withdrawal creation even when the withdrawal ID itself differs. Nonce reservations are consensus-visible and included in the state root.


## M97 — External Deposit Proof Verification Framework

External bridge deposits now have a domain-separated `ExternalDepositProof` commitment covering bridge, source chain, transaction, event/output index, block/slot, finality and proof bytes. A verifier interface provides a common security boundary for Bitcoin, EVM, TRON, TON and Solana adapters. M97 establishes binding and finality checks; chain-specific Merkle/MPT/receipt/BOC/account-state cryptographic proof engines remain adapter-specific follow-up work.


## M99 — Bitcoin Block Header Binding

Bitcoin Merkle proofs are now bound to an 80-byte Bitcoin block header. The header is serialized with Bitcoin little-endian field encoding and hashed with SHA-256d to derive the block hash; the proof requires the header Merkle root to match the verified transaction Merkle root. This prevents a standalone Merkle branch from being treated as a sufficient external inclusion proof.


## M100 — Bitcoin Proof-of-Work Boundary

Bitcoin block headers now have deterministic PoW validation: compact `nBits` decoding, rejection of zero/negative/invalid targets, SHA-256d block hashing and hash-to-target comparison. The header model also exposes explicit previous-block linkage for subsequent chain validation. This is a pure consensus-side verifier and does not perform RPC or network discovery.


## M101 — Bitcoin Header Chain Verification

Bitcoin bridge verification can now validate a contiguous header sequence. Every header must satisfy PoW and each header must reference the exact SHA-256d hash of its predecessor; the requested starting height must also be representable without overflow. This establishes a deterministic header-chain boundary before adding accumulated-chainwork selection.


## M102 — Bitcoin Accumulated Chainwork

Bitcoin header chains now expose deterministic per-header work `floor(2^256 / (target + 1))`, accumulated chainwork, and chainwork comparison for competing validated header sequences. This establishes the basis for selecting the chain with the greatest demonstrated PoW rather than trusting arrival order or raw height alone.


## M102 — Bitcoin Accumulated Chainwork

Bitcoin header chains now expose deterministic per-header work and accumulated chainwork. Work is calculated as `2^256 / (target + 1)` using arbitrary-precision arithmetic, then represented as a fixed 256-bit big-endian value. Competing valid chains can be compared by accumulated work rather than height alone.


## M102 — Bitcoin Accumulated Chainwork

Bitcoin header chains now expose deterministic per-header work `floor(2^256 / (target + 1))`, accumulated chainwork and chainwork comparison. Big-integer arithmetic avoids fixed-width overflow during accumulation. Invalid PoW or broken header linkage prevents a chain from producing usable work.


## M103 — Bitcoin Light-Client Checkpoint

Bitcoin bridge verification now supports a trusted checkpoint containing height, block hash and accumulated chainwork. New headers must directly continue the checkpoint, preserve height continuity and increase total chainwork. This creates a deterministic light-client trust anchor before checkpoint persistence and reorg policy.


## M104 — Persistent Bitcoin Checkpoints

Bitcoin light-client checkpoints are now stored in `ChainState` per bridge, included in the state root, validated by invariants and protected against height/hash rollback. The serde-defaulted registry is backward compatible with pre-M104 state and therefore survives snapshot/restart serialization.


## M105 — Bitcoin Checkpoint / Bridge Policy Binding

Bitcoin checkpoints are now explicitly bound to their bridge security policy and adapter descriptor. The bridge ID, Bitcoin adapter kind, confirmations finality model and required confirmation depth must match the registered policy before the checkpoint can be trusted by the bridge layer.


## M106 — Bitcoin Reorg Detection

Checkpoint advancement now has an explicit reorg boundary. A candidate header sequence that does not directly continue the trusted checkpoint is reported as `BitcoinReorgDetected` and cannot mutate checkpoint state. Only a fully validated continuation with increased chainwork advances the persisted checkpoint.


## M107 — Persistent Bitcoin Reorg Evidence

Detected Bitcoin reorg conflicts are now stored as bounded, deduplicated evidence per bridge. Each record binds the trusted checkpoint to the candidate tip and candidate chainwork through a domain-separated fingerprint. The journal is capped at 32 entries and is included in the state root, so evidence survives restart/snapshot without unbounded state growth.


## M108 — Bitcoin Reorg Severity Policy

Bitcoin reorg evidence can now be classified as Normal, Elevated, or Critical by a persisted bridge policy. A competing chain with greater accumulated chainwork is Critical; a sufficiently deeper competing tip is Elevated; other conflicts are Normal. Classification is informational/policy state only and never mutates or rolls back the trusted checkpoint.


## M109 — Bitcoin Reorg Policy Enforcement

Persisted Bitcoin reorg severity now acts as a bridge safety gate. Elevated or Critical evidence blocks Bitcoin checkpoint advancement and deposit acceptance until the evidence state is recovered/cleared by an explicit state transition. Normal evidence does not block operation. No automatic rollback or fork selection is performed.


## M110 — Bitcoin Reorg Recovery

A blocked Bitcoin bridge can now be recovered through a persisted recovery authorization bound to the exact evidence fingerprint and a monotonic nonce. Recovery requires at least the bridge policy validator quorum and known validator identities. Evidence is retained for audit; recovery does not erase or rewrite the conflict history.


## M111 — Authenticated Bitcoin Reorg Recovery

Bitcoin reorg recovery now carries per-validator Ed25519 signatures. Recovery is accepted only when signatures authenticate against the current validator set and the authenticated count reaches the bridge policy quorum. Validator IDs alone are no longer sufficient authorization.


## M112 — Bitcoin Recovery Canonicality Binding

Recovery authorizations now bind to a canonical Bitcoin height, block hash, and accumulated chainwork. Recovery is rejected if the declared canonical point is below the trusted checkpoint in height or chainwork, preventing recovery from being used to authorize rollback to an older/weaker branch.


## M113 — Bitcoin Canonical Tip Proof

Reorg recovery now supports an explicit `BitcoinCanonicalTipProof`. The recovery tip height, hash and accumulated chainwork must exactly match a contiguous, PoW-validated header chain extending the trusted checkpoint. A claimed canonical tip without a verifiable header chain cannot recover the bridge.


## M114 — Persistent Bitcoin Canonical Tip

Verified Bitcoin canonical tips are now persisted per bridge as independent trust anchors. A tip can only be stored after the supplied header-chain proof validates against the trusted checkpoint, and it cannot regress in height or accumulated chainwork. The canonical-tip registry is included in the state root and survives restart/snapshot.


## M115 — Bitcoin Deposit / Canonical Tip Binding

Bitcoin Merkle deposit proofs can now be validated against the persisted canonical tip. The deposit block must be the first header in a contiguous PoW-validated chain ending exactly at the trusted canonical tip; bridge reorg gates remain active during verification. This prevents deposits from being accepted solely from an isolated Merkle proof or stale branch.


## M116 — Bitcoin Deposit Finality Depth Binding

Bitcoin deposit finality is now derived from the verified canonical tip: `tip_height - deposit_block_height + 1`. The proof's claimed confirmation count must exactly match that derived depth, and the derived depth must satisfy the bridge policy's `min_external_confirmations`. A caller can no longer inflate confirmations by supplying an arbitrary number.


## M117 — Bitcoin UTXO Deposit Binding

Bitcoin UTXO deposits now expose an explicit commitment over transaction hash, output index, amount and scriptPubKey. State validation requires the proof to match the expected UTXO identity and value, uses the persisted canonical tip for finality depth, and keeps the bridge reorg gate active. This prevents an external caller from swapping the transaction output, amount or destination script independently of the proof.


## M118 — Bitcoin UTXO Full Inclusion Boundary

Bitcoin UTXO proofs now carry the raw transaction and cryptographically bind it to the claimed transaction hash using SHA-256d. Deposit validation requires a non-empty Merkle branch and derives the Merkle root from the transaction hash before the existing canonical-tip/finality checks. Full Bitcoin transaction parsing and output-level amount/script extraction remain the next verifier boundary.

## M119 — Bitcoin Raw Transaction Parser + Output Verification

Bitcoin raw transactions are now parsed inside the verifier with legacy and SegWit serialization support. The parser enforces canonical CompactSize encoding, exact transaction consumption, input/output structure, and derives both TXID (without witness data) and WTXID (full serialization). Bitcoin UTXO verification now binds the claimed vout, amount and scriptPubKey to the actual parsed transaction output instead of trusting those fields independently. This completes the transaction-inclusion-to-exact-output boundary; live unspent/spent-state verification remains a separate next boundary.


## M120 — Bitcoin UTXO Spend-State Verification

- Bitcoin headers do not commit a UTXO set, so M120 does not claim a compact UTXO absence proof from headers alone.
- BitcoinUtxoSpendStateProof verifies the exact M119 output binding first.
- The complete inclusion block is scanned, covering same-block spends.
- Every subsequent block through the canonical tip is supplied with its complete raw transaction list.
- Each block's transaction set is parsed, its Merkle root is recomputed with Bitcoin's odd-leaf duplication rule, and the result is checked against the PoW-valid block header.
- Every transaction input is checked for a spend of the target (txid, vout).
- Header linkage is checked from the inclusion block through the canonical tip.
- The claimed tip must exactly match the persisted canonical Bitcoin checkpoint.
- State exposes validate_bitcoin_utxo_spend_state so bridge validation can require the unspent-state proof.
- This is a correctness-first full-block scan and can be large; it is not yet a compact UTXO-accumulator proof and does not replace full Bitcoin script/consensus validation.


## M121 — Compact Bitcoin UTXO Accumulator Boundary

- Adds a domain-separated 321MORE Network UTXO accumulator commitment.
- Adds compact Merkle-style membership proofs for an exact (txid, vout, amount, scriptPubKey) leaf.
- The accumulator is bound to the canonical Bitcoin tip height, block hash and chainwork.
- State can validate an accumulator membership proof against its persisted Bitcoin canonical tip.
- M121 does not claim that this accumulator root is committed by Bitcoin consensus or by the Bitcoin block header.
- A production bridge must authenticate the accumulator root through a separately specified commitment source before treating the compact proof as authoritative.
- M120 remains the fallback correctness proof when no authenticated external UTXO commitment is available.


## M122 — Bitcoin UTXO Commitment Attestation

- Adds a signed attestation for an externally maintained UTXO accumulator root.
- The signed statement binds bridge ID, chain ID, policy ID, sequence, Bitcoin canonical tip height/hash/chainwork and accumulator root.
- Validator signatures are verified against the current validator key set and the bridge validator quorum.
- Validator IDs must be unique and authorized by the attestation itself.
- Attestations are monotonic by sequence per bridge to prevent replay/rollback.
- The attestation is persisted in ChainState and can be queried after registration.
- M122 still does not claim that the accumulator root is a Bitcoin consensus commitment; it authenticates the root as a 321MORE Network bridge commitment.


## M123 — Bitcoin UTXO Accumulator State Transitions

- Replaces the earlier conceptual accumulator fold with a deterministic binary Merkle tree.
- Leaves are canonical UTXO records (txid, vout, amount, scriptPubKey), sorted by leaf hash.
- Odd nodes are duplicated at each tree level; the leaf count is committed in accumulator metadata.
- Adds an explicit UTXO state transition containing spent outpoints and newly created outputs.
- The transition recomputes the complete next UTXO set and therefore the next authenticated root; duplicate, missing-spend and output-collision conditions are rejected.
- The state layer checks that a transition starts exactly from the currently canonical Bitcoin tip and advances one Bitcoin block.
- This is a correctness-first reference transition protocol. It does not pretend Bitcoin headers themselves commit to the UTXO set.


## M124 — Bitcoin Block-Derived UTXO Transitions

- Derives spent outpoints and created outputs directly from parsed Bitcoin block transactions.
- Enforces exactly one coinbase transaction in position zero and rejects null-prevout inputs elsewhere.
- Rejects duplicate transaction IDs and malformed transaction sets.
- Applies transactions sequentially to the previous UTXO state, so valid intra-block spends are supported.
- Every spend must consume an currently available UTXO; every created output is inserted into the resulting state.
- The resulting UTXO Merkle root and leaf count are recomputed from the complete post-block set.
- This keeps the compact commitment path tied to actual Bitcoin transaction data rather than validator-supplied delta lists.


## M125 — Canonical Bitcoin Block Binding

- Binds the M124 UTXO transition to an actual Bitcoin block header.
- The supplied transaction set must reproduce the header Merkle root.
- The header must pass Bitcoin proof-of-work validation.
- The block height must be exactly the canonical tip height plus one.
- The header parent hash must equal the persisted canonical Bitcoin tip hash.
- The resulting block hash must equal the transition next_tip.
- UTXO root and leaf count are recomputed from the transaction set only after header binding succeeds.


## M126 — Compact Bitcoin UTXO Transition Boundary

- Adds transition witnesses and a multiproof boundary so bridge proofs do not inherently require transmitting the complete previous UTXO set.
- Spent outputs are authenticated against the previous UTXO root.
- The protocol explicitly does not claim that spent membership alone is sufficient to derive the next root.
- A complete transition still requires enough authenticated Merkle witness material to reconstruct the affected tree state; the current boundary represents that requirement explicitly.
- This avoids a false compactness claim while leaving the proof format ready for a real Merkle multiproof implementation.


## M127 — Fixed-Key Bitcoin UTXO SMT Updates

- Introduces a V2 fixed-key sparse Merkle tree boundary for compact UTXO updates.
- The key is derived from txid and vout, so deleting one UTXO does not renumber every other leaf.
- Each update step authenticates its old value against the current root and computes the next root after replacing that value.
- Both insertion and deletion are represented by old and new entries, including absence proofs through an empty leaf.
- Steps are verified sequentially, so every intermediate root is authenticated.
- This is the compact update architecture; the earlier sorted-leaf accumulator remains a legacy V1 format.


## M128 — Bitcoin Block to Compact SMT Transition

- Adds a bridge from parsed Bitcoin block transactions to the fixed-key SMT update format.
- The generator derives the exact spent outpoints and created outputs from the block itself.
- Witness steps are checked against that block-derived delta before a compact proof is constructed.
- The state layer first validates the block against the canonical Bitcoin parent and then verifies the compact SMT transition.
- Bitcoin block hash and UTXO SMT root remain separate commitments; the block hash is never incorrectly treated as the UTXO root.


## M129 — Automatic Bitcoin UTXO SMT Witness Generation

- Adds deterministic empty-subtree hashes for the 256-level fixed-key SMT.
- Adds a complete UTXO snapshot abstraction with root calculation and per-key proof generation.
- Adds automatic compact update-proof construction by diffing previous and next snapshots.
- Only changed UTXO keys become update steps; unchanged state is represented by Merkle siblings.
- The resulting proof can be independently verified from previous root to next root without shipping the complete UTXO snapshot.


## M130 — End-to-End Bitcoin UTXO Commitment Pipeline

- Adds BitcoinUtxoBlockCommitment as the end-to-end artifact for one canonical Bitcoin block.
- The builder validates the block structure, applies every transaction sequentially to the previous UTXO snapshot, creates the next snapshot, and automatically builds the compact SMT update proof.
- The commitment binds block height and block hash to previous and next UTXO roots.
- State APIs now validate and build the commitment only from the canonical Bitcoin successor block.
- This closes the reference pipeline from Bitcoin block data to an authenticated compact UTXO state transition.


## M131 — Bitcoin UTXO Commitment Attestation Binding

- Extends the validator attestation to sign both the previous and next UTXO roots.
- The attestation is bound to the exact successor Bitcoin block height and block hash.
- Signature/quorum verification is separated from canonical-tip verification because a successor block is intentionally one height beyond the persisted tip.
- State validation requires exact agreement between the commitment and attestation: bridge, block height, block hash, previous root and next root.
- This prevents reusing a valid quorum signature for a different Bitcoin block transition.


## M132 — Attested Bitcoin UTXO Transition Lifecycle

`apply_bitcoin_utxo_attested_commitment(...)` is the state transition boundary for a quorum-attested Bitcoin UTXO update.

It atomically:
1. verifies the candidate block is the exact successor of the persisted canonical tip;
2. binds block hash/height to the UTXO commitment and validator attestation;
3. rejects a transition whose previous UTXO root does not equal the last attested root;
4. rejects attestation sequence replay;
5. advances the canonical Bitcoin checkpoint and persisted UTXO attestation together.

The block transition remains adapter-specific: a Bitcoin block hash is never treated as an SMT root.


## M133 — Cross-Chain Finality Envelope

Introduces a chain-neutral `CrossChainFinalityEnvelope` carrying bridge/chain identity, sequence, finalized block or checkpoint identity, previous/current state roots, proof kind, and a domain-separated proof hash.

Supported proof kinds currently include Bitcoin UTXO, Ethereum, Tron, TON, Solana and a generic finality adapter. The envelope is deliberately only a common boundary: each chain must provide its own proof construction and verification rules.

Bitcoin validation additionally requires the candidate finality height to be the successor of the persisted canonical tip and rejects unchanged state roots and sequence replay.


## M134 — Cross-Chain Proof Adapters

Adds the `CrossChainProofAdapter` boundary. Each chain supplies its own proof semantics while State routes validation through the adapter selected by `CrossChainProofKind`.

Current adapters: Bitcoin UTXO, Ethereum/EVM, and Tron. Ethereum/EVM and Tron adapters intentionally provide only structural envelope validation at this stage; they do not claim transaction/event finality until their chain-specific proof verifiers are implemented.


## M135 — Ethereum Proof Boundary

Adds explicit Ethereum proof structures for transaction/receipt inclusion metadata and finalized-block metadata. `EthereumTransactionProof::verify_binding` binds a transaction proof to the common finality envelope by exact block height, block hash and proof hash.

This is intentionally a proof boundary rather than a claim of Ethereum consensus verification: RLP/receipt-trie verification and Ethereum PoS finality certificate verification remain separate implementation steps.


## M136 — Ethereum Receipt MPT Verification

Ethereum receipt proofs now use raw RLP-encoded Merkle-Patricia-Trie nodes and Keccak-256 node references. Verification checks the proof path from `receiptsRoot`, supports both hashed and inline child nodes, validates compact hex-prefix paths, and binds the proof to the transaction index and receipt value.

This is an inclusion-proof verifier only. Ethereum PoS consensus/finality certificates and the full receipt semantic decoder remain separate layers.


## M137 — Ethereum PoS Finality Boundary

Adds `EthereumFinalityProof` for finalized execution-block metadata, state root, validator-set root, checkpoint ancestry and finality evidence. The proof is bound to the common envelope by exact finalized height/hash, state root and proof hash.

The current boundary deliberately does not claim full Ethereum consensus verification: SSZ checkpoint proofs, sync-committee/validator-set validation, BLS aggregate signature verification and fork-choice rules remain explicit follow-up layers.


## M138 — Ethereum SSZ Merkle Primitives

Adds SSZ-oriented SHA-256 Merkle primitives and generalized-index proofs. `EthereumSszMerkleProof` verifies a leaf against a trusted SSZ root using the generalized-index branch, while chunking and deterministic power-of-two merkleization provide the base for beacon-state/checkpoint proofs.

Ethereum execution MPT continues to use Keccak-256; SSZ consensus structures use SHA-256 as required by Ethereum's consensus-layer hash tree.


## M139 — Ethereum Validator / Sync Committee SSZ Proofs

Adds validator membership and sync-committee SSZ Merkle proof structures. Proofs use the consensus-layer SHA-256 SSZ tree, generalized indices, and explicit binding to a finalized checkpoint root.

This milestone proves tree membership only. It does not yet verify validator signatures, committee participation, or BLS aggregate signatures; those are M140 cryptographic checks.


## M140 — Ethereum BLS12-381 Sync Committee Verification

Adds BLS12-381 verification through `blst`, including aggregate verification for participating sync-committee public keys. `EthereumSyncCommitteeSignature` commits to the finalized signing root, validator indices and participation bits, rejects malformed/duplicate indices and requires a 96-byte aggregate signature.

The state boundary additionally requires the signature signing root to equal the expected finalized checkpoint signing root. Committee membership/validator-set authorization remains a separate SSZ proof boundary.


## M141 — Ethereum End-to-End Proof Pipeline

Adds `EthereumEndToEndProof`, composing the Ethereum consensus and execution proof layers into one verification boundary: finalized checkpoint metadata, SSZ committee membership, BLS aggregate signature, execution block binding, receipt MPT inclusion, and exact envelope binding.

State exposes `validate_ethereum_end_to_end_proof(...)` as the unified validation entry point. The layers remain independently verifiable so execution proofs and consensus proofs cannot silently substitute for one another.


## M205 — Bitcoin Bridge Mint Admission Binding

The consensus `BridgeMint` admission path now consumes the persisted Bitcoin canonical tip. For Bitcoin mainnet deposits it requires the deposit height to be buried under the bridge policy's minimum confirmation depth and rejects stale or inconsistent UTXO attestation tips. M204 remains the cryptographic/header/fork verifier; M205 connects that finality boundary to actual mint transaction admission.
## M206 — Bitcoin UTXO Attestation Required for Mint Admission

Bitcoin BridgeMint admission now requires a state-held, quorum-authenticated `BitcoinUtxoCommitmentAttestation` for the bridge. The attestation must bind to the exact persisted canonical Bitcoin tip, use the same source chain ID, and satisfy the bridge validator quorum. M205 established the mandatory canonical-tip/finality boundary; M206 makes the authenticated UTXO commitment layer mandatory as well, without changing the BridgeMint transaction payload format.
## M207 — Bitcoin Mainnet Chain-ID Binding

The Bitcoin BridgeMint admission gate now uses the protocol-defined `BITCOIN_MAINNET_CHAIN_ID` instead of a legacy numeric literal. This keeps M205/M206 Bitcoin finality and UTXO-attestation enforcement aligned with the canonical bridge network identity used by the external-proof verifier.


## M208 — Bitcoin UTXO Attestation Chain-ID Binding

Bitcoin UTXO commitment attestations are now rejected at state registration unless their signed chain_id is exactly BITCOIN_MAINNET_CHAIN_ID.

This closes the state-level identity gap left by M205–M207: an attestation cannot be registered into the Bitcoin UTXO state path under an unrelated external chain ID. Runtime mint admission already binds the same value through the protocol-defined Bitcoin chain ID.


## M209 — Bitcoin State-Proof Chain-ID Binding

Bitcoin UTXO state validators now require the protocol-defined Bitcoin mainnet chain ID across accumulator membership proofs, spend-state proofs, and direct UTXO deposit proofs.

This keeps all state-level Bitcoin proof entry points aligned with the M207/M208 chain identity binding and prevents a non-Bitcoin proof from entering a Bitcoin-specific state path.


## M210 — Bitcoin Adapter Chain-ID Binding

Bitcoin checkpoint-to-adapter validation now requires BridgeAdapterDescriptor.chain_id to equal the protocol-defined BITCOIN_MAINNET_CHAIN_ID. A Bitcoin adapter can no longer bind a checkpoint through an unrelated nonzero chain ID.

Regression coverage verifies both rejection of an Ethereum chain ID and acceptance of the canonical Bitcoin mainnet ID. No bridge payload or prior M-module architecture was changed.


## M211 — Bitcoin Merkle Deposit Chain-ID Binding

The Bitcoin canonical-tip Merkle deposit validator now requires BitcoinMerkleProof.chain_id to equal the protocol-defined BITCOIN_MAINNET_CHAIN_ID before canonical-header verification.

Regression coverage verifies that a proof carrying an unrelated chain ID is rejected at the Bitcoin-specific state boundary. The generic BitcoinMerkleProof verifier remains unchanged.


## M212 — Bitcoin UTXO Transition Chain-ID Binding

Bitcoin UTXO accumulator transition entry points now require the transition chain_id to equal the protocol-defined BITCOIN_MAINNET_CHAIN_ID. This applies to both block-bound transition validation and standalone accumulator transition validation.

Regression coverage verifies rejection of an Ethereum chain ID at both Bitcoin-specific state boundaries. No transition payload structure or previous M-module architecture was changed.


## M213 — Bitcoin UTXO Root Continuity

Bitcoin UTXO block-commitment and compact-transition validators now bind each new previous_root to the persisted Bitcoin UTXO attestation accumulator_root when one exists.

This prevents a standalone proof path from introducing a disconnected UTXO state root between authenticated Bitcoin commitments. Existing payload structures and prior M-module architecture remain unchanged.


## M214 — Bitcoin Finality Adapter Chain-ID Binding

The Bitcoin UTXO cross-chain finality adapter now requires the protocol-defined BITCOIN_MAINNET_CHAIN_ID before accepting a Bitcoin finality envelope.

Regression coverage verifies rejection of a non-Bitcoin chain ID and acceptance of the protocol Bitcoin chain ID. No envelope payload structure or prior M-module architecture was changed.


## M215 — Bitcoin Cross-Chain Finality State Binding

The state-level cross-chain finality validator now binds Bitcoin UTXO finality envelopes directly to the protocol-defined BITCOIN_MAINNET_CHAIN_ID. This removes the invalid dependency on a non-existent chain_id field in BridgeSecurityPolicy and keeps the state boundary consistent with the Bitcoin finality adapter.

Regression coverage verifies rejection of a non-Bitcoin chain ID. No envelope payload structure or prior M-module architecture was changed.


## M216 — Bitcoin Finality Envelope Root Continuity

When a prior authenticated Bitcoin UTXO attestation exists, a Bitcoin UTXO finality envelope must carry the same accumulator root as its previous_state_root. This binds the next finality transition to the authenticated prior UTXO state instead of allowing a disconnected state root.

Regression coverage verifies rejection of a finality envelope with an unrelated previous UTXO root.


## M217 — Bitcoin Finality Sequence Continuity

Bitcoin UTXO cross-chain finality envelopes now require a contiguous sequence transition: when a prior authenticated attestation exists, the next envelope sequence must equal the previous sequence plus one.

This prevents gaps in the authenticated finality state machine while preserving the existing envelope structure and all previous M-module invariants.


## M218 — Bitcoin Finality Bridge Policy Integrity

Bitcoin UTXO finality state validation now requires the referenced bridge security policy to be internally valid and bound to the same bridge_id as the finality envelope.

This closes a policy-integrity gap without inventing a future-block hash check that the current envelope payload cannot substantiate.


## M219 — Bitcoin UTXO Attestation Sequence Continuity

Bitcoin UTXO commitment attestations now require contiguous sequence progression. Both attestation registration and authenticated commitment application reject a sequence gap and require the next sequence to equal the previous sequence plus one.

This aligns the persisted Bitcoin UTXO attestation state machine with the strict finality-envelope sequence invariant introduced in M217, preventing authenticated UTXO state from skipping intermediate transitions.


## M220 — Bitcoin Finality Sequence Overflow Safety

Bitcoin UTXO attestation and cross-chain finality sequence transitions now reject the terminal u64::MAX state from being reused. Sequence progression requires a real +1 transition, preventing saturating arithmetic from accepting a repeated maximum sequence value.


## M221 — Bitcoin UTXO Attestation Active-Validator Binding

Bitcoin UTXO commitment attestations are now authenticated against the active validator set rather than the broader validator registry. Any attestation naming a signer outside the current active set is rejected before signature verification.


## M222 — Bitcoin Commitment Attestation Active-Validator Binding

The Bitcoin UTXO commitment-application path now enforces the same active-validator binding as attestation registration. Attestations naming validators outside the current active set are rejected before commitment application and signature quorum validation.


## M223 — Bitcoin Reorg Recovery Active-Validator Binding

Bitcoin reorg recovery quorum authentication now uses the active validator set. Signatures from validators that are no longer active cannot satisfy the recovery quorum, keeping reorg recovery aligned with the current bridge security authority.


## M224 — Bridge Authorization Active-Validator Binding

Bridge pause authorization and bridge mint/withdrawal approval quorum checks now authenticate against the active validator set rather than the broader validator registry. This prevents validators that are no longer active from satisfying current bridge security quorums.


## M225 — Bridge Approval Admission Hardening

Stored bridge message approval sets are now canonicalized and restricted to the active validator set at admission. Validator IDs must be strictly ascending and every approval signer must belong to the active set, preventing inactive or duplicate/non-canonical approval records from entering bridge state.


## M226 — Withdrawal Quorum Active-Validator Binding

Bridge withdrawal approval quorum authentication now uses the active validator set. A validator remaining in the historical registry but removed from the active set cannot satisfy the current withdrawal quorum.


## M227 — Bridge Pause Authorization Canonicalization

Emergency pause authorization sets now require strictly ascending validator IDs before quorum authentication. Duplicate or non-canonical signer entries are rejected, keeping the persisted emergency-control evidence deterministic.


## M228 — Withdrawal Policy Boundary Enforcement

Bridge withdrawal approval now enforces the stored bridge security policy before quorum processing: the policy must be valid, the bridge must not be emergency-paused, the withdrawal amount must satisfy deposit/withdrawal limits, and the withdrawal nonce must meet the configured nonce floor. This closes an authorization-path bypass where a request could otherwise reach approval despite violating policy limits.


## M229 — Withdrawal Request Policy Admission

Bridge withdrawal requests are now checked against the stored security policy at creation time. Invalid policy state, emergency pause, amount limits, and nonce-floor violations are rejected before a withdrawal enters the request lifecycle, complementing the M228 approval-time enforcement.


### M230 — threshold withdrawal approvals require authenticated evidence

Withdrawal approval in `Threshold` and `QuorumAndThreshold` modes no longer trusts the persisted `threshold_signatures` counter by itself. The counter must not exceed the number of independently authenticated, active-validator approvals bound to the withdrawal intent; otherwise approval is rejected. This closes a forged threshold-count bypass without changing the existing approval payload format.


### M231 — overflow-safe bridge policy quorum validation

`BridgeSecurityPolicy::validate()` now evaluates the validator supermajority rule in a wider integer type, preventing `u16` multiplication overflow at extreme validator counts. The policy format and quorum rule remain unchanged; only the arithmetic domain is hardened.


### M232 — Bitcoin UTXO attestation binds chain identity and chainwork

The direct Bitcoin UTXO commitment-attestation validation path now requires the protocol Bitcoin chain ID and exact canonical-tip chainwork, in addition to the existing bridge, height, block-hash, root-continuity, commitment, and active-validator signature checks. This prevents a structurally valid attestation from being rebound to another chain or a weaker/different canonical chainwork value.


### M233 — Bitcoin next-tip chainwork verification

The attested Bitcoin UTXO commitment-application path now derives the submitted block's proof-of-work work from its header and requires the attested next-tip chainwork to equal the persisted canonical chainwork plus that block's work. This closes the gap between a valid PoW block and an independently supplied cumulative-chainwork value without comparing next-tip work directly to the previous canonical tip.


### M234 — Reorg gate on Bitcoin UTXO commitment application

The Bitcoin UTXO attested-commitment application path now passes through the existing Bitcoin reorg safety gate before advancing canonical state. If the latest reorg evidence is Elevated/Critical and has not been authenticated by the matching recovery record, the commitment cannot advance the canonical tip. This closes a direct state-path bypass of reorg recovery controls.


### M235 — Reorg evidence checkpoint binding

Bitcoin reorg evidence is now admitted only when it is bound to the bridge's persisted checkpoint: matching checkpoint height/hash, a strictly later candidate tip, greater candidate chainwork, and a valid reorg policy are required. This prevents independently fabricated or stale reorg evidence from becoming an unlock target for recovery.


## M235 — Bitcoin UTXO validation reorg gate

Bitcoin UTXO commitment construction and validation paths now pass through the same reorg safety gate as canonical-commitment application. During an unresolved Elevated/Critical Bitcoin reorg, block commitments, compact UTXO transitions, accumulator proofs, spend-state proofs, and transition validation are rejected until matching recovery evidence is present. This prevents a reorged canonical tip from being used through a secondary proof-validation path.


## M236 — Bitcoin checkpoint/canonical-tip persistence reorg gate

Bitcoin checkpoint registration and canonical-tip persistence now pass through the reorg safety gate. An unresolved Elevated/Critical reorg therefore cannot be bypassed by replacing the checkpoint anchor or persisting a new canonical tip through a direct state-persistence path; recovery remains the explicit path for resolving the reorg.


### M237 — Bitcoin reorg recovery validator identity binding
- Fixed Bitcoin reorg recovery quorum authentication to index active validator consensus keys by the authenticated `validator_id`, rather than by the raw consensus public key.
- This aligns recovery signatures with the validator identity used by `BitcoinReorgRecovery` and prevents valid active-validator recovery signatures from being ignored.
- Added a regression test confirming the recovery key map is keyed by validator ID.


### M238 — Bitcoin BridgeMint reorg safety gate
- Exposed the Bitcoin bridge safety gate to runtime admission and made Bitcoin `BridgeMint` transactions fail while an unresolved reorg is active.
- This closes the runtime bypass where mint admission could read the persisted canonical tip directly without checking the reorg recovery gate.
- Added a state regression test for the public Bitcoin bridge safety gate.


### M239 — Bitcoin mint attestation uses active validators
- Bitcoin `BridgeMint` runtime admission now authenticates UTXO attestations against the active validator set, matching state-level quorum enforcement.
- This removes the remaining runtime path where a validator present only in the historical/full registry could contribute to a mint quorum.
- Added a regression test for active-validator key resolution.


### M240 — BridgeMint authorization uses active validators
- Runtime `BridgeMint` authorization now resolves validator keys from the active validator set rather than the full validator registry.
- This closes the remaining authorization path where a validator removed from the active set could still authenticate a mint approval.
- Added a regression test distinguishing active validators from registry-only validators.


### M241 — State BridgeMint execution uses active validators
- The state execution path for `BridgeMint` now resolves authorization keys from the active validator set, eliminating the full-registry bypass that remained below runtime admission.
- Bitcoin BridgeMint execution also reuses the persisted Bitcoin bridge safety gate before authorization.
- Added a regression test distinguishing active validators from registry-only validators.


### M242 — Consensus-state Bitcoin BridgeMint gate
- State-level `BridgeMint` execution now enforces the persisted Bitcoin canonical tip, confirmation boundary, and quorum-authenticated UTXO attestation before minting Bitcoin-backed assets.
- This mirrors the runtime admission boundary at the authoritative state execution layer, preventing a direct consensus execution path from bypassing Bitcoin finality checks.
