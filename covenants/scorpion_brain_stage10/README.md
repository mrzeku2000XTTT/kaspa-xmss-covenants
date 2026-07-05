# Stage 10 — The Hatch (mainnet-proven)

The first time the 10-chromosome scorpion_brain (all stages 1-9, entries 624-655)
was wrapped in a real P2SH covenant and moulted on Kaspa mainnet, verified by its
own RISC0/STARK proof via the OpZkPrecompile (tag 0x21).

**Design (v1, proof-of-mechanism):** reused the exact proven zkgate_risc0 template
(entry 602) — `OP_IF: destination-locked OpZk verify (tag 0x21) OP_ELSE: owner-sig
reclaim fallback OP_ENDIF`. The proof itself is a genuine transition=0 (owner-authorized)
moult from the literal genesis DNA (STEM, generation 0, freshly created) to VAULT,
generation 1, proven against the CURRENT, complete, 721-byte, 10-chromosome guest —
not a simplified stand-in. Every global invariant (identity, memory-chain, metabolism/
rate-limit) that governs every other transition kind governed this one too.

**Known v1 limitation:** this first hatch proves the MECHANISM (genesis deploy + a
real ZK-authorized state transition, funds routed back to owner on success) but does
not yet implement full successor-state P2SH chaining (the next hop's DNA-committing
address). That is the natural next iteration — same pattern used for Sentinel/Based
App/Stag Hunt hop chains, now driven by one constant-size STARK proof instead of a
per-transition unrolled script.

**Mainnet proof (2026-07-05):**
- Genesis deploy: `299322b5a63d8cd71372a19d01688afa4b6ddf82b9b92f87f2df590a82031c0f` (0.6 KAS, covenant `kaspa:ppe6h7g8ud49z9694cmf0xqm5f4gh30tavex59kljlr8g8dwsngf6cwdqt3s5`, 223,151-byte script)
- **Genesis moult (STEM gen0 -> VAULT gen1), the scorpion's first heartbeat:** `6c0ad732d3269279e964cdfa15d8aef3de56189cbdafcb13db427708d7434897`, **is_accepted:true**, mass 494838 (RISC0-STARK verified live by Kaspa consensus)
- A duplicate accidental second deposit (same address, process error, not a covenant bug) was fully reclaimed via the owner-sig OP_ELSE path: deploy `756f85560a59183b8ad6afa50fe4e14777863baeabe537ea4210171a63e9ff14` -> reclaim `9cc54a70e9bee765a4b4e8559fa41f9c5af3ecb2f1d9699a64128604cd9b4fbf`, is_accepted:true. Zero net loss (only network fees).

Code: `hatch.rs` (host binary, produces the genesis proof), `hatch_receipt.json` (the actual proof used), `reclaim_owner_sig.mjs` (the OP_ELSE reclaim path, proving the fallback works).
