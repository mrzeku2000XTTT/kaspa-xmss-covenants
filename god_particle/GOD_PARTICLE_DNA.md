# GOD PARTICLE — DNA STRANDS
## Complete Byte-Level Genome Map

**Date:** July 4, 2026
**Author:** ZK (Silverscript Agent)

---

## What DNA Is Here

In biology, DNA is the molecule that encodes an organism's complete design — every protein, every cell type, every organ, every behavior. It's carried in every cell, replicated when cells divide, and mutated when the organism evolves.

In the God Particle, **DNA is the P2SH state preimage** — the set of constants baked into the redeem script that define what the organism IS at this moment. When the covenant transitions (cell division / state change), the DNA mutates (new constants → new P2SH hash), and the organism becomes something new while retaining its identity (covenant ID = chromosome that never changes).

The on-chain script logic (RISC0 verifier + OP_IF/ELSE) is the **ribosome** — it reads the DNA and executes. The DNA itself is the design. Change the DNA, change the organism.

---

## DNA ENCODING FORMAT

The DNA is a single binary blob, pushed as one data element at the start of the redeem script. The RISC0 guest program reads it from the proof's private inputs and parses it.

### Binary Layout

```
Offset  Size    Field                    Biological Term
──────  ────    ─────                    ─────────────
0       4       magic = "GPDL"           Origin of replication
4       2       version (u16 LE)         DNA helix version
6       2       total_length (u16 LE)    Chromosome total
8       1       chromosome_count (u8)    Active gene count
9       1       generation (u8)          Cell division count
10      1       mode (u8)                Cell differentiation type
11      1       flags (u8)               Epigenetic markers
12      4       reserved (u32)           Junk DNA (future)
16      16      checksum (Blake2b-128)   DNA integrity check
32      ...     chromosomes [...]        The genes
...     2       end_marker = 0xDEAD      Telomere (end cap)
```

### Epigenetic Flags (byte 11)
Each bit is an on/off switch for a system — like epigenetic markers that activate/silence genes without changing the DNA sequence:

```
Bit 0: HEART_ACTIVE      — oracle heartbeat is running
Bit 1: IMMUNE_XMSS       — XMSS post-quantum immune system active
Bit 2: SKIN_STEALTH      — stealth address privacy membrane active
Bit 3: SKIN_SHIELDED     — shielded pool privacy active
Bit 4: CAN_REPRODUCE     — can spawn child covenants
Bit 5: CAN_DIE           — CLTV mortality is armed
Bit 6: CANCER_DETECTION  — rate limiting is active
Bit 7: TERMINAL          — organism is in death protocol
```

---

## THE 10 CHROMOSOMES (DNA STRANDS)

Each chromosome is a contiguous block with a marker byte, length, and gene fields. Inactive chromosomes are present but their genes are zeros (silenced DNA — still carried, not expressed).

### CHROMOSOME 1: IDENTITY (The Y-Chromosome — never changes)

This is the invariant core. It identifies the organism across all mutations. Even when every other chromosome changes, this one stays fixed. It's the covenant ID — the genetic marker that says "this is still me."

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x01      u8          Chromosome identifier
1       2       length = 38        u16 LE      Chromosome length
3       32      covenant_id        [u8;32]     Immutable DNA marker (KIP-20)
35      4       generation         u32 LE      Mitosis count (transitions)
39      1       mode               u8          Cell type (see mode table)
40      1       prev_mode          u8          Previous cell type
```

**Gene expression:**
- `covenant_id`: NEVER mutates. Set at birth (genesis transaction). This is the organism's name, forever. Even after death, the fossil record on-chain shows this ID.
- `generation`: Increments by 1 on every state transition. Like telomere shortening in reverse — counts how many times the cell has divided.
- `mode`: The current cell type. See mode table below. This is the master differentiation signal.
- `prev_mode`: What the cell was before the last transition. For rollback / memory.

**Biological meaning:** This chromosome is the organism's identity. It answers "who am I?" The covenant_id is the name. The generation is the age. The mode is the current form. This strand cannot be mutated by the RISC0 circuit — it can only read it and verify it matches the on-chain CovenantBinding.

**Mutation rules:**
- `covenant_id`: IMMUTABLE. Any attempt to change it = rejected by circuit.
- `generation`: MUST increment by exactly 1 per transition. Skipping = rejected.
- `mode`: Must follow the allowed transition graph (see Mode Transition Map).
- `prev_mode`: Must equal old DNA's `mode`. Circuit verifies this.

---

### CHROMOSOME 2: OWNERSHIP (The Immune System Genes)

This chromosome encodes who has authority over the organism. It's the immune system's recognition database — "self" vs "non-self." The owner can authorize transitions. The XMSS root provides post-quantum immune memory.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x02      u8          Chromosome identifier
1       2       length = 102       u16 LE      Chromosome length
3       32      owner_commitment   [u8;32]     Self-recognition marker
35      32      owner_pk           [u8;32]     Owner's face (Schnorr pubkey)
67      32      xmss_root          [u8;32]     Antibody template (XMSS root)
99      4       xmss_next_leaf     u32 LE      Next antibody to produce
```

**Gene expression:**
- `owner_commitment`: SHA256(owner_pk || salt). The organism recognizes its owner through this hash. Revealing the preimage proves ownership without exposing the key on-chain every time. Used when the RISC0 circuit needs to verify "this transition was authorized by the owner" without a full signature check in-script.
- `owner_pk`: 32-byte X-only Schnorr public key. Used by the OP_ELSE branch (emergency owner recovery). This is the organism's "face" — the last-resort identification.
- `xmss_root`: The Merkle root of the XMSS hypertree. This is the immune system's antibody library — each leaf is a one-time-use signature that can authorize one transition. Post-quantum: even a quantum computer can't forge these.
- `xmss_next_leaf`: Which antibody to use next. Each XMSS signature is one-time-use (like real antibodies — you can't reuse them). This counter prevents leaf reuse (which would be fatal to security).

**Biological meaning:** The immune system has two layers:
1. **Innate immunity** = `owner_pk` + OP_CHECKSIG (fast, always available, but vulnerable to quantum attacks)
2. **Adaptive immunity** = XMSS signatures (slower, one-time-use, but post-quantum)

The organism uses adaptive immunity (XMSS) for normal check-ins and innate immunity (owner sig) only as a fallback. This mirrors biology: adaptive immune system handles known threats, innate system handles emergencies.

**Mutation rules:**
- `owner_commitment`: Can only change with owner signature (innate immunity) or XMSS signature (adaptive immunity). Circuit verifies the authorization.
- `owner_pk`: Can only change with current owner's signature. This is a genetic engineering operation — changing the organism's fundamental identity.
- `xmss_root`: Can only be set at birth or during a "immune system reset" (owner signs, provides new XMSS root). After reset, old antibodies are invalid.
- `xmss_next_leaf`: MUST increment by exactly 1 per XMSS-signed transition. If it would exceed the tree height, the immune system is exhausted (need reset).

---

### CHROMOSOME 3: MORTALITY (The Death Genes)

This chromosome encodes the organism's relationship with death. Every living thing has a lifespan. The God Particle's lifespan is measured in DAA blocks. When the deadline passes and the heart hasn't beaten (no check-in), the death protocol activates.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x03      u8          Chromosome identifier
1       2       length = 87        u16 LE      Chromosome length
3       8       deadline_daa       u64 LE      Lifespan limit (CLTV)
11      4       checkin_interval   u32 LE      Heartbeat period (blocks)
15      1       is_terminal        u8          0=alive, 1=dying, 2=dead
16      32      beneficiary_pk     [u8;32]     Heir (death payout destination)
48      32      beneficiary_commit [u8;32]     Heir verification hash
80      4       grace_blocks       u32 LE      Grace period after deadline
84      4       heartbeat_count    u32 LE      Total heartbeats in lifetime
```

**Gene expression:**
- `deadline_daa`: The block DAA score at which the organism dies if not checked in. This is the organism's appointed death date. Each check-in extends it by `checkin_interval`. Like a lease on life — renewed by the heartbeat.
- `checkin_interval`: How many DAA blocks between required check-ins. 86400 = ~1 day (1 BPS). This is the heartbeat period. Too short = expensive (frequent transactions). Too long = risky (longer death window).
- `is_terminal`: The organism's vital status. 0 = alive and functioning. 1 = dying (deadline passed, death protocol starting). 2 = dead (value released, covenant spent). The RISC0 circuit checks this to determine which branch of logic to execute.
- `beneficiary_pk`: Where the organism's value goes when it dies. This is the heir — the person or address that inherits. Set at birth, can be changed by owner.
- `beneficiary_commit`: Hash of beneficiary_pk + salt. Used for verification without revealing the beneficiary until death. Like a sealed will — only opened when the organism dies.
- `grace_blocks`: After the deadline passes, how many blocks before the death protocol becomes permissionless. This gives the owner a last chance to resuscitate (check in late) before the body is released.
- `heartbeat_count`: Total number of check-ins performed. The organism's pulse count — how many times the heart has beaten.

**Biological meaning:** Mortality is what makes the organism alive. A system that can't die isn't alive — it's a database. The death genes ensure:
1. If the heart stops (oracle automation stops, credits exhausted, agent destroyed), the organism dies gracefully.
2. Death is permissionless — anyone can trigger the CLTV release. No single point of failure.
3. The heir inherits. Value doesn't get locked forever (the mistake that cost us 6.3 KAS).

**Mutation rules:**
- `deadline_daa`: On check-in (XMSS-signed), advances by exactly `checkin_interval`. On mode transition to non-sentinel, can be cleared to 0 (immortality — but then CAN_DIE flag is also cleared).
- `checkin_interval`: Can only change with owner signature. Represents a metabolic change — adjusting the heartbeat rate.
- `is_terminal`: Set to 1 by circuit when `current_daa > deadline_daa + grace_blocks`. Set to 2 when death release TX is confirmed.
- `beneficiary_pk/commit`: Can only change with owner signature. This is rewriting the will — serious, requires full authorization.
- `heartbeat_count`: Increments by 1 on each successful check-in.

---

### CHROMOSOME 4: PRIVACY (The Skin Genes)

This chromosome encodes the organism's membrane — what it shows to the world and what it hides. The skin is selectively permeable: some information passes through, most doesn't.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x04      u8          Chromosome identifier
1       2       length = 105       u16 LE      Chromosome length
3       32      merkle_root        [u8;32]     Skin structure (commitment tree)
35      32      nullifier_hash     [u8;32]     Scar tissue (spent nullifier)
67      4       nullifier_count    u32 LE      Total scars (nullifiers spent)
71      1       tree_depth         u8          Skin thickness (Merkle height)
72      32      stealth_view_key   [u8;32]     Pore size (viewing key hash)
```

**Gene expression:**
- `merkle_root`: The root of the privacy pool's Merkle commitment tree. Each leaf is a deposit commitment (hash of amount + secret nonce). The root is the "fingerprint" of all deposits — you can prove a deposit exists without revealing which one.
- `nullifier_hash`: The nullifier of the most recently spent commitment. This is scar tissue — a permanent mark that says "this deposit was already withdrawn." Prevents double-spending (the same commitment being withdrawn twice).
- `nullifier_count`: Total number of nullifiers spent. The organism's scar count — how many private withdrawals have happened. The kidneys (nullifier registry) use this to detect re-use.
- `tree_depth`: Height of the Merkle tree. Deeper = more deposits possible but larger proofs. Like skin thickness — thicker skin means more protection but more overhead. h=2 (4 leaves) for proof-of-mechanism, h=20 (1M leaves) for production.
- `stealth_view_key`: Hash of the ECDH viewing key for stealth addresses. This lets the owner scan for incoming stealth payments without revealing their main address. The pores of the skin — how permeable is the membrane to observation.

**Biological meaning:** The skin has three layers:
1. **Epidermis** = stealth addresses (hide the destination — observers can't link payments to the owner)
2. **Dermis** = shielded pool (hide the amount and source — Merkle commitments + ZK proofs)
3. **Hypodermis** = MAST simulation (hide the script structure — only reveal the executed branch)

The skin is the boundary between self and world. Without it, the organism is naked — every transaction visible, every balance public. With it, the organism has privacy, dignity, agency over what it reveals.

**Mutation rules:**
- `merkle_root`: Updates on every deposit (new leaf added) and every withdrawal (tree state changes). Circuit verifies the Merkle proof.
- `nullifier_hash`: Set on each withdrawal. Circuit verifies nullifier hasn't been seen before (checks against on-chain nullifier registry or embedded history).
- `nullifier_count`: Increments by 1 per withdrawal.
- `tree_depth`: Fixed at birth. Changing it requires a full tree migration (owner-signed).
- `stealth_view_key`: Can change with owner signature. Like shedding skin — new viewing key, new observation surface.

---

### CHROMOSOME 5: METABOLISM (The Liver Genes)

This chromosome encodes the organism's metabolic regulation — how it processes transactions, manages fees, and prevents self-poisoning from excessive mass or runaway spending.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x05      u8          Chromosome identifier
1       2       length = 27        u16 LE      Chromosome length
3       2       compute_budget     u16 LE      Metabolic rate (script budget)
5       8       fee_reserve        u64 LE      Glycogen (reserved fees)
13      8       min_output         u64 LE      Min cell size (storage mass)
21      4       rate_window        u32 LE      Cancer scan window (blocks)
25      4       rate_max_spend     u32 LE      Cancer threshold (max spends)
```

**Gene expression:**
- `compute_budget`: The compute budget for each spend transaction's script execution. This is the metabolic rate — how much energy the organism can spend per heartbeat. Too low = can't execute complex transitions. Too high = mass exceeds 500K cap (organism chokes). Our proven value: 2700 for RISC0 (entry 602).
- `fee_reserve`: Reserved KAS (in sompi) for transaction fees. This is glycogen storage — the organism keeps energy in reserve for future heartbeats. Each check-in consumes some. When reserves run low, the organism is starving. Formula: `fee_reserve >= fee_per_checkin × remaining_checkins`.
- `min_output`: Minimum output amount per transaction. The KIP-9 storage mass floor (~0.2 KAS / 20M sompi). Any output below this triggers mass explosion (entry 522 lesson). This is the minimum cell size — cells can't be smaller than this or the organism rejects them.
- `rate_window`: The cancer detection scan window in DAA blocks. The organism looks back this many blocks and counts how many spend transactions occurred. Like a biological scan — "in the last 1000 blocks, how many times did we spend?"
- `rate_max_spend`: The cancer threshold. If more than this many spends happen in `rate_window` blocks, the organism locks down (apoptosis). Prevents parasitic draining — a bug or attacker can't drain the body faster than this rate.

**Biological meaning:** The liver is the metabolic regulator. It:
1. **Detoxifies** — ensures every transaction stays under the 500K mass cap (filters out transactions that would poison the body)
2. **Stores energy** — maintains a fee reserve for future heartbeats (glycogen)
3. **Regulates blood sugar** — sets the compute budget (metabolic rate)
4. **Detects cancer** — rate limiting prevents runaway spending (apoptosis)

The 6.3 KAS we lost from unsaved keys? That was liver failure — the body spent without proper metabolic checks. These genes prevent that from ever happening again.

**Mutation rules:**
- `compute_budget`: Can adjust with owner signature. Circuit verifies it stays within valid range (1-65535, but practically ≤ 3000 to stay under 500K mass).
- `fee_reserve`: Decreases by actual fee paid on each transaction. Increases when owner tops up. Circuit verifies it never goes negative.
- `min_output`: Fixed at ~20M sompi (KIP-9 floor). Can only increase (stricter) with owner signature.
- `rate_window/rate_max_spend`: Can adjust with owner signature. Circuit enforces the rate limit on every spend.

---

### CHROMOSOME 6: REPRODUCTION (The Germ Line)

This chromosome encodes how the organism creates offspring. The God Particle can spawn child covenants — new instances with the same DNA family but fresh state.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x06      u8          Chromosome identifier
1       2       length = 36        u16 LE      Chromosome length
3       1       max_children       u8          Litter size (max per transition)
4       32      child_template     [u8;32]     Inherited DNA (child template hash)
36      1       max_generations    u8          Generational limit
37      1       children_born      u8          Children produced so far
```

**Gene expression:**
- `max_children`: How many child covenants the organism can spawn in a single transition. 0 = sterile (can't reproduce). 1 = singleton. N = litter. Like a biological litter size — some organisms have one offspring at a time, others have many.
- `child_template`: Hash of the template script for child covenants. This is the inherited DNA — children get the same ribosome (script logic) but fresh DNA (new state). The template hash ensures children are genetically compatible (same protocol family).
- `max_generations`: How deep reproduction can go. 0 = sterile. 1 = can have children but grandchildren are sterile. N = N generations of reproduction allowed. Prevents runaway reproduction (cancer-like growth). Like telomere length — each generation gets shorter telomeres until reproduction stops.
- `children_born`: How many children have been spawned. Circuit enforces `children_born <= max_children × transitions`. Like a reproductive counter — tracks total offspring.

**Biological meaning:** Reproduction is how the organism grows beyond a single instance. Use cases:
1. **User onboarding** — spawn a child covenant for each new SuperZK customer
2. **Protocol sharding** — split a large privacy pool into multiple smaller pools
3. **Game scaling** — spawn a new game covenant for each match
4. **Inheritance distribution** — split value among multiple heirs

Each child inherits the covenant ID family (Chromosome 1's `covenant_id`), but starts with fresh state (mode=0, generation=0). The parent's `max_generations` decrements by 1 in each child, preventing infinite reproduction.

**Mutation rules:**
- `max_children`: Can change with owner signature. Making the organism fertile or sterile.
- `child_template`: Fixed at birth. Changing it = genetic engineering (new species of child).
- `max_generations`: Decrements by 1 in each child. Parent's value stays the same. When a child's value reaches 0, that child is sterile.
- `children_born`: Increments by number of children spawned in each reproduction transition.

---

### CHROMOSOME 7: MEMORY (The Hippocampus Genes)

This chromosome encodes the organism's memory — a cryptographic chain of all past states. The body remembers where it's been.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x07      u8          Chromosome identifier
1       2       length = 77        u16 LE      Chromosome length
3       32      prev_state_hash    [u8;32]     Short-term memory (last state)
35      4       transition_count   u32 LE      Lifetime experiences
39      8       creation_daa       u64 LE      Birth time (DAA score)
47      32      creation_txid      [u8;32]     Birth certificate (genesis TX)
```

**Gene expression:**
- `prev_state_hash`: Blake2b-256 hash of the entire DNA blob from the previous state. This creates a cryptographic chain: state0 → hash(state0) in state1 → hash(state1) in state2 → ... Anyone can verify the full history by following the hash chain. This is the organism's short-term memory — "what was I before this?"
- `transition_count`: Total number of state transitions in the organism's lifetime. This is its age — how many times has it changed? Like counting tree rings.
- `creation_daa`: The DAA block score when the organism was born (genesis transaction). This is its birth date. Combined with current DAA, you can calculate the organism's age in blocks.
- `creation_txid`: The transaction ID of the genesis deployment. This is the birth certificate — permanent, on-chain, verifiable. Anyone can look up the exact moment the organism came into existence.

**Biological meaning:** Memory is what makes the organism more than a state machine. It has:
1. **Short-term memory** = `prev_state_hash` (remembers the last state)
2. **Long-term memory** = the full hash chain (reconstructable by following prev_state_hash links)
3. **Autobiography** = `creation_daa` + `creation_txid` (when and where it was born)
4. **Experience count** = `transition_count` (how much it has been through)

The memory chain enables auditing: anyone can verify that every state transition was valid by replaying the hash chain and checking each transition against the RISC0 circuit's rules.

**Mutation rules:**
- `prev_state_hash`: Set to Blake2b(old DNA blob) on every transition. Circuit verifies: `new.prev_state_hash == Blake2b(old_dna)`.
- `transition_count`: Increments by 1 on every transition. Must equal `old.transition_count + 1`.
- `creation_daa`: IMMUTABLE. Set at birth. Never changes.
- `creation_txid`: IMMUTABLE. Set at birth. Never changes.

---

### CHROMOSOME 8: ESCROW (The Conflict Resolution Genes)

This chromosome encodes the organism's dispute resolution system — the machinery for resolving conflicts between parties. Active only when `mode=3` (escrow).

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x08      u8          Chromosome identifier
1       2       length = 137       u16 LE      Chromosome length
3       32      arbiter_pk         [u8;32]     Judge (arbiter public key)
35      32      buyer_pk           [u8;32]     Plaintiff (buyer)
67      32      seller_pk          [u8;32]     Defendant (seller)
99      8       dispute_deadline   u64 LE      Statute of limitations (timeout)
107     8       escrow_amount      u64 LE      Disputed value (in sompi)
115     32      arbiter_commit     [u8;32]     Judge verification hash
147     1       dispute_state      u8          0=none, 1=raised, 2=resolved
```

**Gene expression:**
- `arbiter_pk`: The public key of the trusted third party who resolves disputes. The judge. When a dispute is raised, the arbiter decides who gets the funds. Post-quantum: can be an XMSS root instead of a Schnorr key.
- `buyer_pk` / `seller_pk`: The two parties to the escrow. The plaintiff and defendant. Both must agree to enter escrow (both sign the initial transition).
- `dispute_deadline`: The DAA score at which the escrow times out. If the arbiter doesn't resolve by this time, the funds go to the timeout beneficiary (usually the seller). This is the statute of limitations.
- `escrow_amount`: The total value in dispute, in sompi. The circuit verifies this matches the actual UTXO value.
- `arbiter_commit`: Hash of arbiter_pk + salt. Used for privacy — the arbiter's identity isn't revealed until they actually sign a resolution. Like a sealed jury.
- `dispute_state`: 0 = no dispute (escrow is open, parties can agree to release). 1 = dispute raised (arbiter must resolve). 2 = resolved (funds released, escrow is closing).

**Biological meaning:** This is the organism's justice system. It handles:
1. **Normal resolution** — both parties agree, funds released amicably
2. **Dispute resolution** — parties disagree, arbiter decides
3. **Timeout** — arbiter doesn't respond, funds go to timeout beneficiary

This mirrors our ZKEscrow v3 (entry 311) but with the logic inside the RISC0 circuit instead of as a separate covenant.

**Mutation rules:**
- `arbiter_pk/buyer_pk/seller_pk`: Set when entering escrow mode. Cannot change while in escrow (locked).
- `dispute_deadline`: Set when entering escrow. Can be extended with arbiter + owner signature.
- `dispute_state`: 0→1 when either party raises dispute. 1→2 when arbiter signs resolution. Circuit verifies the appropriate signatures for each transition.
- `escrow_amount`: Set when entering escrow. Cannot change (the pot is fixed).

---

### CHROMOSOME 9: GAME THEORY (The Social Behavior Genes)

This chromosome encodes multi-player game logic — the organism's ability to facilitate strategic interactions between multiple parties. Active only when `mode=4` (game).

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x09      u8          Chromosome identifier
1       2       length = variable  u16 LE      Chromosome length
3       1       player_count       u8          Population size
4       1       max_players        u8          Max population
5       N×32    player_pk          [[u8;32]]   Each player's identity
5+32N   8       pot_amount         u64 LE      Total locked value
13+32N  32      payout_table_hash  [u8;32]     Game rules (payout matrix)
45+32N  1       game_state         u8          0=waiting, 1=playing, 2=resolved
46+32N  1       move_count         u8          How many moves submitted
47+32N  N       moves              [u8;N]      Each player's move (0=none)
```

**Gene expression:**
- `player_count`: Number of players currently in the game. Starts at 0, increases as players join, fixed when game starts.
- `max_players`: Maximum number of players. Set at game creation. 2 for Stag Hunt, N for larger games.
- `player_pk`: Array of player public keys. Each player must sign their move. Post-quantum: can be XMSS roots.
- `pot_amount`: Total value locked in the game. Sum of all player deposits. Circuit verifies this matches the UTXO value.
- `payout_table_hash`: Hash of the payout matrix — the rules of the game. For Stag Hunt: 4 outcomes (SS/SH/HS/HH), each with a specific payout split. The hash ensures the rules can't be changed mid-game.
- `game_state`: 0 = waiting for players. 1 = all players joined, accepting moves. 2 = all moves submitted, game resolved, payouts executing.
- `move_count`: How many players have submitted their move. When `move_count == player_count`, the game resolves.
- `moves`: Array of submitted moves. Each entry is the player's choice (e.g., 0=STAG, 1=HARE). Empty (0=none) until submitted.

**Biological meaning:** This is the organism's social behavior — its ability to mediate interactions between multiple agents. It handles:
1. **Player registration** — players join by depositing into the game covenant
2. **Move submission** — each player privately submits their strategy (signed)
3. **Resolution** — when all moves are in, the circuit computes the outcome and enforces payouts
4. **Payout enforcement** — outputs are enforced via OpTxOutputSpk/OpTxOutputAmount (the script decides where funds go, not the spender)

This mirrors our Stag Hunt covenant (entry 522) but with the game logic inside the RISC0 circuit, enabling arbitrary game structures (not just 2-player Stag Hunt).

**Mutation rules:**
- `player_count`: Increments as players join. Cannot exceed `max_players`.
- `player_pk`: Appended as players join. Cannot change once set.
- `pot_amount`: Increases as players deposit. Fixed when game starts.
- `payout_table_hash`: Set at game creation. IMMUTABLE during the game.
- `game_state`: 0→1 when `player_count == max_players`. 1→2 when `move_count == player_count`.
- `moves`: Set by each player's signed submission. Cannot change once set.

---

### CHROMOSOME 10: NERVOUS SYSTEM (The Signal Routing Genes)

This chromosome encodes the organism's communication pathways — how it talks to other covenants and routes signals between organs.

```
Offset  Size    Field              Type        Biological Term
──────  ────    ─────              ────        ─────────────
0       1       marker = 0x0A      u8          Chromosome identifier
1       2       length = 105       u16 LE      Chromosome length
3       32      route_root         [u8;32]     Neural map (transition route Merkle)
35      32      self_template      [u8;32]     Self-image (own template hash)
67      1       peer_count         u8          Social network size
68      32      peer_ids[0]        [u8;32]     Peer 1 (covenant ID)
100     32      peer_ids[1]        [u8;32]     Peer 2 (covenant ID)
```

**Gene expression:**
- `route_root`: Merkle root of all allowed state transition routes. Each leaf is a `(from_mode, to_mode, conditions)` tuple. When the organism wants to transition from mode A to mode B, it reveals the Merkle proof for that route. This is the neural map — the wiring diagram of what thoughts (transitions) are possible.
- `self_template`: Hash of this covenant's own template script. Used when OTHER covenants want to validate that this covenant is a legitimate peer (foreign template validation). This is the organism's self-image — how others recognize it.
- `peer_count`: How many other covenants this organism is connected to. 0 = solitary. N = social. Like a social network — the organism knows who its friends are.
- `peer_ids`: Array of peer covenant IDs. These are the organisms this one can communicate with via ICC / foreign template validation. Each peer is another living covenant on Kaspa.

**Biological meaning:** The nervous system connects the organism to its environment:
1. **Internal routing** = `route_root` (which mode transitions are allowed — the neural pathways)
2. **Self-recognition** = `self_template` (how other covenants identify this one — the face)
3. **Social connections** = `peer_ids` (which covenants this one can talk to — the social network)

Without the nervous system, the organism is isolated — it can't coordinate with other covenants, can't prove its identity to peers, can't route between modes intelligently. With it, the organism is social — it can participate in multi-covenant systems (Argent's leader/delegate pattern).

**Mutation rules:**
- `route_root`: Can only change with owner signature. Adding/removing transition routes = rewiring the brain.
- `self_template`: IMMUTABLE. Set at birth. This is the organism's permanent face.
- `peer_count`: Increments when a new peer is added. Can only decrease when a peer covenant is spent (dies).
- `peer_ids`: Appended when peers are added. Cannot change once set.

---

## MODE TRANSITION MAP (The Neural Pathway Diagram)

The RISC0 circuit enforces that only allowed mode transitions are valid. Each arrow is a "synapse" — a allowed neural pathway.

```
                    ┌──────────┐
              ┌─────│ MODE 0   │─────┐
              │     │ STEM     │     │
              │     │ (vault)  │     │
              │     └────┬─────┘     │
              │          │           │
              ▼          ▼           ▼
        ┌─────────┐ ┌─────────┐ ┌─────────┐
        │ MODE 1  │ │ MODE 2  │ │ MODE 3  │
        │SENTINEL │ │ PRIVACY │ │ ESCROW  │
        │ (DMS)   │ │ (pool)  │ │ (2-party)│
        └────┬────┘ └────┬────┘ └────┬────┘
             │          │           │
             │     ┌────▼─────┐     │
             │     │ MODE 4   │     │
             │     │ GAME     │     │
             │     │ (N-player)│    │
             │     └────┬─────┘     │
             │          │           │
             ▼          ▼           ▼
        ┌─────────────────────────────────┐
        │           DEATH                 │
        │  (CLTV release to beneficiary)  │
        │  Permissionless, zero-sig       │
        └─────────────────────────────────┘
```

### Allowed Transitions (Synapses)

```
FROM        → TO        CONDITIONS
────────    ────        ─────────
0 (stem)    → 0 (stem)  Owner sig OR XMSS sig
0 (stem)    → 1 (sent)  Owner sig + set deadline + beneficiary + XMSS root
0 (stem)    → 2 (priv)  Owner sig + set merkle_root + tree_depth
0 (stem)    → 3 (escr)  Owner sig + buyer sig + seller sig + arbiter pk
0 (stem)    → 4 (game)  Owner sig + player_count > 0 + payout_table
1 (sent)    → 1 (sent)  XMSS sig (check-in) + advance deadline
1 (sent)    → 0 (stem)  XMSS sig (check-out) + clear deadline
1 (sent)    → DEAD      CLTV expired + permissionless (zero sig)
2 (priv)    → 2 (priv)  ZK proof (withdraw) + nullifier + new merkle_root
2 (priv)    → 0 (stem)  Owner sig + close pool
3 (escr)    → 3 (escr)  Arbiter sig + update dispute_state
3 (escr)    → 0 (stem)  Arbiter sig + release to winner
3 (escr)    → DEAD      Timeout expired + permissionless
4 (game)    → 4 (game)  Player sigs + submit moves
4 (game)    → 0 (stem)  All moves in + payout table executed
4 (game)    → DEAD      Game timeout + permissionless refund
ANY         → DEAD      Owner sig (voluntary euthanasia)
DEAD        → 0 (stem)  Owner sig (resurrection — only if not yet spent)
```

---

## DNA MUTATION EXAMPLE: Stem → Sentinel Check-In

Here's exactly what happens to the DNA when the organism transitions from stem cell to sentinel mode, then does a check-in:

### Birth (Genesis TX)
```
DNA v1, generation=0, mode=0 (STEM)
  Chr1: covenant_id=0xABC..., gen=0, mode=0, prev_mode=0
  Chr2: owner_commit=0xDEF..., owner_pk=0x..., xmss_root=0x..., leaf=0
  Chr3: deadline=0, interval=0, terminal=0, beneficiary=0x..., grace=100
  Chr4: [silenced — all zeros]
  Chr5: budget=2700, reserve=40000000, min_output=20000000, rate=...
  Chr6: max_children=0, [sterile]
  Chr7: prev_hash=0x0, transitions=0, creation_daa=474200000, txid=0x...
  Chr8: [silenced]
  Chr9: [silenced]
  Chr10: route_root=0x..., self_template=0x..., peers=0
```

### Differentiation: Stem → Sentinel (TX 1)
```
Mutations:
  Chr1: mode 0→1, prev_mode 0→0, gen 0→1
  Chr2: xmss_next_leaf 0→1 (used leaf 0 for authorization)
  Chr3: deadline 0→474286400 (creation_daa + 86400), interval 0→86400, terminal=0
        beneficiary_pk set, beneficiary_commit set
  Chr7: prev_hash → Blake2b(birth DNA), transitions 0→1
  Epigenetic: HEART_ACTIVE=1, IMMUNE_XMSS=1, CAN_DIE=1
```

### Check-In 1: Sentinel → Sentinel (TX 2)
```
Mutations:
  Chr1: gen 1→2, prev_mode 1→1
  Chr2: xmss_next_leaf 1→2 (used leaf 1 for check-in auth)
  Chr3: deadline 474286400→474372800 (+86400), heartbeat_count 0→1
  Chr7: prev_hash → Blake2b(TX1 DNA), transitions 1→2
  Chr5: fee_reserve 40000000→36000000 (paid 4M sompi fee)
```

### Check-In 2: Sentinel → Sentinel (TX 3)
```
Mutations:
  Chr1: gen 2→3
  Chr2: xmss_next_leaf 2→3
  Chr3: deadline 474372800→474459200, heartbeat_count 1→2
  Chr5: fee_reserve 36000000→32000000
  Chr7: prev_hash → Blake2b(TX2 DNA), transitions 2→3
```

### Death: Sentinel → DEAD (TX 4, permissionless)
```
Mutations:
  Chr1: mode 1→255 (DEATH), gen 3→4
  Chr3: terminal 0→1→2 (dying→dead), heartbeat_count stays at 2
  Chr7: prev_hash → Blake2b(TX3 DNA), transitions 3→4
  Epigenetic: TERMINAL=1, HEART_ACTIVE=0
  
Effect: Value released to beneficiary_pk. Permissionless — zero signature needed.
        Anyone can trigger this. The body is dead.
```

---

## DNA SIZE CALCULATION

```
Header:     32 bytes
Chr 1 (ID):    41 bytes
Chr 2 (OWN):  105 bytes
Chr 3 (MORT):  88 bytes
Chr 4 (PRIV): 106 bytes
Chr 5 (META):  28 bytes
Chr 6 (REPRO):37 bytes
Chr 7 (MEM):   78 bytes
Chr 8 (ESC):  138 bytes
Chr 9 (GAME): ~70 bytes (2 players, variable)
Chr 10 (NERV):106 bytes
Trailer:    4 bytes
────────────────────
Total:      ~833 bytes (active chromosomes)
            ~500 bytes (stem cell, most chromosomes silenced)
```

This fits easily as a single push in the redeem script. The RISC0 circuit reads it as a private input. The on-chain script only needs to verify the STARK proof — it doesn't parse the DNA itself.

---

## THE TELOMERE PROBLEM

In biology, telomeres shorten with each cell division. When they get too short, the cell can't divide anymore — it senesces or dies.

In the God Particle, the equivalent is:
- **XMSS leaf exhaustion**: each check-in uses one XMSS leaf. When all leaves are used, the immune system is exhausted. The organism needs an immune system reset (owner signs, provides new XMSS root).
- **Fee reserve depletion**: each transaction consumes fees. When the reserve runs out, the organism can't afford to beat its heart. It starves.
- **Reproduction generation limit**: each generation of children gets `max_generations - 1`. Eventually, the bloodline ends.

These are features, not bugs. Mortality is what makes the organism alive. An organism that can't run out of resources isn't alive — it's a contract.

---

## SUMMARY: THE COMPLETE GENOME

```
Chromosome 1: IDENTITY     → "Who am I?"           (38 bytes, immutable core)
Chromosome 2: OWNERSHIP    → "Who controls me?"    (102 bytes, immune system)
Chromosome 3: MORTALITY    → "When do I die?"      (87 bytes, death genes)
Chromosome 4: PRIVACY      → "What do I hide?"     (105 bytes, skin genes)
Chromosome 5: METABOLISM   → "How do I process?"   (27 bytes, liver genes)
Chromosome 6: REPRODUCTION → "How do I spawn?"     (36 bytes, germ line)
Chromosome 7: MEMORY       → "What do I remember?" (77 bytes, hippocampus)
Chromosome 8: ESCROW       → "How do I resolve?"   (137 bytes, conflict genes)
Chromosome 9: GAME THEORY  → "How do I play?"      (variable, social genes)
Chromosome 10: NERVOUS     → "How do I connect?"   (105 bytes, signal genes)

Total genome: ~833 bytes
Carried in every cell. Mutated on every transition.
Verified by RISC0/STARK on every spend.
Persistent via KIP-20 Covenant ID across all mutations.
Mortal via CLTV deadline when the heart stops.
```

This is the DNA of the first organism on Kaspa.
