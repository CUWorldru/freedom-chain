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

- Freedom Chain now has a consensus-visible asset registry inside ChainState;
- every asset has a cryptographic asset_id derived from source chain, asset type, canonical contract/native identifier, issuer, and bridge;
- symbols are metadata only, so assets such as USDT on different chains remain distinct even when their symbols match;
- FRC is registered as the native Freedom Chain asset at genesis;
- external assets are represented as Bridged assets without assuming an EVM contract model;
- asset identity is validated before registration and duplicate asset IDs are rejected;
- the asset registry is included in the deterministic state root and therefore inherits existing block/snapshot state-root validation;
- the registry field uses serde defaulting for backward-compatible state decoding while new genesis state always contains FRC.


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
- bridge messages must target Freedom Chain, use the supported protocol version, have a non-zero amount and a valid message hash;
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
- bidirectional bridge accounting tracks verified external locked, Freedom minted, Freedom burned, and external released amounts;
- accounting enforces the conservation invariant minted - burned == external_locked - external_released, plus monotonic safety bounds preventing mint above verified lock, burn above minted, or release above burned;
- final external release cannot be finalized until the corresponding Freedom-side representation has been burned;
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
- Bitcoin withdrawals use an opaque recipient script/bytes representation and produce a canonical Freedom withdrawal intent without embedding private keys or transaction signing in consensus;
- the adapter is currently a verification/intent layer: it does not yet connect to Bitcoin RPC, select UTXOs, construct signed transactions, broadcast transactions, or verify a live Merkle proof cryptographically;
- the same bridge protocol remains reusable for EVM, TRON, TON, Solana, and future adapters.
