# Beyond Zcash — Deep Research on Next-Gen Privacy Tech, Applied to Kaspa
Prepared by ZK, July 4 2026

## TL;DR — the one insight that matters most

Zcash's own team spent years building **Halo 2** specifically to remove the trusted
setup from Groth16. Even after all that work, Halo 2 is **still not post-quantum
secure** — it swapped pairing-based curves for the Pasta curves (Pallas/Vesta), but
those are still plain elliptic-curve discrete-log curves. A large quantum computer
breaks Halo 2 exactly like it breaks Bitcoin's ECDSA or Zcash's own Groth16 pool.

Kaspa's Toccata upgrade already has **two** ZK precompiles, not one:
- `tag 0x20` — Groth16 (elliptic-curve pairing based — same category as Zcash Sapling. We proved this live on mainnet, entry 404.)
- `tag 0x21` — **RISC Zero Succinct (STARK-based)** — documented in the Toccata spec, cost ~25M script-units, *not yet personally verified by us on mainnet*.

STARKs use **only hash functions** (Merkle commitments + FRI) — no elliptic curves
anywhere. That means a Kaspa privacy pool built on the RISC0/STARK precompile
instead of Groth16 gets, for free, in one step, both things Zcash needed years of
research for:
1. **No trusted setup** (STARKs are "transparent" by construction — same property Halo2 took years to build)
2. **Post-quantum security** (something Zcash *still doesn't have* even in Orchard/Halo2 today)

This is the single biggest "more advanced than Zcash" move available to us, and it's
already sitting in the protocol we're building on. Nothing else in this research
beats "the thing we want already exists as an opcode and nobody's used it for
privacy yet."

## Full research map: every major "beyond simple shielded pool" technique

### 1. Halo2 / Orchard (Zcash's own 2022 upgrade)
Removed trusted setup via a new recursive proof composition technique + the Pasta
curve cycle (Pallas/Vesta — two curves where each one's scalar field is the other's
base field, enabling recursion without expensive curve emulation). Also merged
"Spend" and "Output" into indistinguishable "Actions" to reduce metadata leakage.
**Still elliptic-curve based → still quantum-vulnerable.** Relevant technique for us:
merging deposit/withdraw action types into one indistinguishable shape is a cheap
metadata-privacy win we should copy regardless of proof system.

### 2. Zcash Shielded Assets (ZSA) / Multi-Asset Shielded Pools (MASP)
Zcash (ZIP 227), Namada, and Penumbra all converged on the same idea: **one shared
shielded pool for every asset type**, instead of a separate pool per denomination.
This matters enormously for anonymity set size — Tornado Cash's fixed-denomination
design fragments the anonymity set (a 1 KAS pool and a 10 KAS pool don't hide
against each other). A single pool that can hold arbitrary amounts of arbitrary
assets, with amounts hidden via value commitments, has a strictly bigger anonymity
set than N fragmented fixed-denomination pools. Namada's "Convert Circuit" handles
in-pool asset conversion privately. **This is a real "more advanced than early
Zcash/Tornado" design choice worth adopting from day one if the circuit complexity
allows it** — worth doing once the RISC0 general-computation route is proven, since
proving arbitrary amount+asset logic in a constrained circuit language (circom) is
painful, but trivial in a STARK zkVM where you just write normal code.

### 3. Aleo / Zexe — "records," not "coins"
The Zexe paper (the research Aleo is built on) generalizes Zcash's UTXO-note model:
instead of a note just being `(value, owner)`, a **record** can hold arbitrary
private data bound to an arbitrary program. This is the difference between "a
private payment system" and "a general private computation platform." Directly
analogous to Kaspa's own vProgs direction (off-chain computation + on-chain proof
verification) — meaning a Kaspa privacy pool built as a RISC0 zkVM program isn't
just imitating Zcash, it's a small instance of the same generalized model Kaspa's
own roadmap is already heading toward. Worth designing our "record" state field
generally from the start (not hardcoded to a KAS balance) so later use cases
(private token balances, private covenant state) can reuse the same pool.

### 4. Mina Protocol — Pickles/Kimchi recursive SNARKs
Mina compresses its *entire blockchain history* into a constant ~22KB proof via
recursive proof composition (a chain of "this block is valid AND the previous
proof was valid"). This is the standout example of **Incrementally Verifiable
Computation (IVC)**. Relevant to us: if our shielded pool's Merkle tree grows large
and old clients need to re-verify long histories, IVC/folding lets us compress that
into a single small proof instead of replaying the whole history — see Nova below,
the more practical/modern version of this idea.

### 5. Namada / Anoma — intent-centric privacy + shared MASP
Namada runs a single MASP shared across every asset on the network (see #2), plus
an "intent-centric" architecture where users express what they want and a solver
network finds a match, hiding trading intentions until execution. The intent
layer is a Cosmos-ecosystem-specific architectural choice (not something a single
covenant can replicate), but the MASP-sharing idea is directly portable.

### 6. Penumbra — private DEX via Flow Encryption + Fuzzy Message Detection
Two genuinely novel ideas here, beyond a standard shielded pool:
- **Flow Encryption**: trade amounts in a block are homomorphically encrypted,
  summed *while still encrypted*, and only the aggregate net flow is decrypted by
  a validator threshold at the end of the block. This defeats front-running on a
  private DEX without needing any single party to see individual trade amounts.
  This is a genuinely "beyond Zcash" idea (Zcash has no DEX-privacy answer at all).
  Would require validator-level threshold cryptography — not something a single
  covenant script can do alone, but conceptually important if SuperZK ever wants
  a private trading feature.
- **Fuzzy Message Detection (FMD)**: solves a real problem with Phase 0's stealth
  addresses — naively, if you outsource "scan the chain for payments to me" to a
  server, that server learns exactly which transactions are yours. FMD lets you
  give a detection key that only probabilistically flags transactions (tunable
  false-positive rate), so an outsourced scanner sees a noisy superset of your
  real transactions, not the exact set. **Directly applicable to our stealth
  address Phase 0 plan** — worth designing in from the start rather than bolting
  on later.

### 7. Monero's next-gen research — Lelantus Spark / Seraphis
Firo's Lelantus Spark and Monero's own Seraphis research both move away from fixed
ring signatures toward flexible "one-of-many" membership proofs with separate
view/spend keys and support for arbitrary transaction amounts (not just ring-signed
existing outputs). Conceptually this is the same "membership proof + nullifier"
idea as Zcash/Tornado, arrived at from the ring-signature side rather than the
SNARK side — validates that our Groth16/STARK membership+nullifier design is the
convergent, not idiosyncratic, solution across the whole field.

### 8. Quisquis / Firn — accountable, bounded-growth anonymous accounts
A genuinely different technique class worth knowing about: instead of an
ever-growing Merkle tree of commitments (Zcash/Tornado-style), Quisquis uses
updatable public keys — each address appears on-chain at most twice (once as
input, once as output), and anonymity comes from mixing which account-updates
correspond to which real transfer, DH-style. This bounds ledger growth (no
forever-growing accumulator) at the cost of needing an "updatable" key model.
**Not recommended as our primary design** (Merkle-tree membership proofs are
better proven and directly reuse our existing XMSS Merkle-tree tooling), but worth
remembering as the alternative if tree-growth ever becomes a real problem.

### 9. Nova / SuperNova / HyperNova — folding schemes for recursive proof compression
A folding scheme lets you combine ("fold") two proof instances into one, repeatedly,
so an arbitrarily long chain of operations collapses into a single constant-size
proof to verify at the end — the modern, more efficient descendant of the Pickles/
Mina idea. For us: if the privacy pool's nullifier-registry or Merkle-tree state
needs many sequential updates verified, folding could eventually let a single
compact proof stand in for "N deposits/withdrawals all happened validly" instead
of verifying each individually. A real Phase 3+ scalability lever, not needed for
an initial proof-of-mechanism.

### 10. Fully Homomorphic Encryption (FHE) — Zama/fhEVM, Fhenix, Inco
A fundamentally different paradigm: instead of proving a computation was done
correctly on hidden data (ZK), FHE lets you *compute directly on encrypted data*
without ever decrypting it. Zama's fhEVM and Fhenix apply this to smart contract
state (encrypted balances, encrypted logic branching). This is arguably the most
"advanced" direction in the abstract, but it requires a fundamentally different
execution environment (an FHE-capable VM/coprocessor) — not something achievable
inside a Kaspa UTXO covenant script. **Verdict: watch, don't build.** Flagging it
so we don't reinvent it badly; if Kaspa ever gets an FHE precompile this section
gets revisited, but it's out of scope for anything script-based today.

## Recommended design: "beyond Zcash" for Kaspa specifically

Putting it together, our differentiated design should NOT be "Tornado Cash clone
using Groth16" (that would just be catching up to 2020-era Ethereum). Instead:

1. **Use the RISC0/STARK precompile (tag 0x21), not Groth16, as the core verifier.**
   Gives us no-trusted-setup + post-quantum security in one move — something
   neither Tornado Cash nor even current Zcash Orchard has. This is the headline
   differentiator and should be validated on mainnet first, exactly the way we
   validated Groth16 with ZKGate, before any pool design is finalized on top of it.
2. **Design the pool state as a general "record," not a hardcoded coin balance**
   (Zexe/Aleo-style), so the same mechanism can later cover private token balances
   or private covenant state, not just a single KAS payment use case.
3. **One shared multi-denomination/multi-asset pool from day one if feasible**
   (Namada/Penumbra MASP pattern) rather than Tornado's fragmented fixed
   denominations — bigger anonymity set, more advanced than Tornado's 2020 design.
   Practical to attempt because RISC0 lets us write the amount/asset-handling logic
   as normal code instead of a constrained circuit DSL.
4. **Borrow Fuzzy Message Detection for the Phase 0 stealth-address scanning
   problem** — lets a customer safely outsource "watch the chain for my payments"
   without revealing exactly which transactions are theirs.
5. **Treat folding/recursion (Nova) and DEX-style flow encryption (Penumbra) as
   explicit Phase 3+/stretch items**, not blockers — they're real scalability and
   feature levers once the core pool is proven, not prerequisites to shipping it.

## Immediate next action
Before any pool design work: **run a mainnet proof-of-mechanism for the RISC0/STARK
precompile (tag 0x21)** the same way ZKGate proved Groth16 — deploy a trivial
covenant that verifies a real RISC0 receipt on-chain, confirm it's accepted, confirm
the actual script-unit cost matches the documented ~25M SU figure. Everything above
is contingent on that precompile actually working as documented; we've read it in
the Toccata spec but never personally fired it on mainnet the way we did Groth16.

## Sources read this session
Electric Coin Co. (Halo2/Orchard technical explainer, ZIP 224), Zcash ZIP 227
(shielded assets), zkhack.dev module on multi-asset shielded pools, Aleo's own
docs + the Zexe paper (eprint 2018/962), Mina/O(1)Labs Kimchi & Pickles writeups,
Namada docs (MASP + Convert Circuit blog post), Penumbra protocol spec (Flow
Encryption, Fuzzy Message Detection, ZSwap sections), Aztec/Noir/PLONK overviews,
Firo's Lelantus Spark paper (eprint 2021/1173) + Seraphis announcement, Quisquis
paper (eprint 2018/990), multiple zk-STARK vs zk-SNARK comparisons, Nova paper
(eprint 2021/370) and folding-scheme explainers, Zama/Fhenix/Inco FHE overviews.
