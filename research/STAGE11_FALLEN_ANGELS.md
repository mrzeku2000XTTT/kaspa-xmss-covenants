# Stage 11 — "To Fallen Angels" (Fall & Redemption Chromosome)

## The gap this closes

Right now death is handled entirely OUTSIDE the ZK brain. The P2SH wrapper has
an `OP_ELSE` CLTV branch (the "stinger") that releases funds permissionlessly
once a deadline lapses — but the RISC0 guest itself has no concept of dying.
Worse: if metabolism (Chr9) detects starvation or cancer, the guest just
**panics** — no valid successor proof can be produced at all. The organism
doesn't die gracefully, it just gets stuck, and the only way out is the
external owner-sig or CLTV fallback outside the ZK circuit entirely.

`is_terminal` / `beneficiary_commit` / `grace_blocks` / `heartbeat_count` have
been sitting unused in Chromosome 2 since the Sentinel days — this is where
they were always headed.

## The idea

A scorpion that fails doesn't just vanish. It **falls** — cast down from
active life into a restricted, watched state — and gets one real chance at
**redemption** before permanent death. Only if it squanders that chance does
it fossilize forever. This is provable in ZK, permissionless where it should
be (nobody should need the owner's secret to prove the organism starved), and
still requires the owner's proof-of-life to climb back out.

## New modes

```
MODE_FALLEN = 6   // cast down, restricted, on probation
MODE_FOSSIL = 7   // permanent grave — terminal, no transitions ever again
```

## New DNA fields (Chromosome 11, appended after byte 721 → DNA_LEN becomes 741)

| Offset    | Field                  | Type | Meaning |
|-----------|------------------------|------|---------|
| 721       | `fall_reason`          | u8   | 1=starvation, 2=cancer/rate-limit, 3=deadline-lapsed, 4=owner-voluntary |
| 722–729   | `fallen_at_daa`        | u64  | DAA the fall occurred |
| 730–737   | `redemption_deadline`  | u64  | `fallen_at_daa + redemption_window`; miss it → fossilize |
| 738       | `redemption_progress`  | u8   | consecutive healthy proof-of-life heartbeats while fallen |
| 739       | `redemption_threshold` | u8   | owner-tunable policy — heartbeats needed to ascend |
| 740       | `fossilized`           | u8   | 0/1, permanent once set |

## Transition graph additions

```
(STEM,   FALLEN)  // permissionless — anyone can prove starvation/cancer/deadline-lapse
(VAULT,  FALLEN)  // same
(SENTINEL, FALLEN)// same
(FALLEN, FALLEN)  // redemption heartbeat — owner proves control (XMSS-style), redemption_progress += 1
(FALLEN, VAULT)   // ASCENSION — only if redemption_progress >= redemption_threshold
                  //   AND current_daa <= redemption_deadline
                  //   owner must also top up fee_reserve (resupply on rebirth)
(FALLEN, FOSSIL)  // permissionless finalization — only if current_daa > redemption_deadline
                  //   AND redemption_progress < redemption_threshold
                  //   this is where funds actually release to beneficiary_commit
(FOSSIL, *)       // NOTHING. Terminal. Chitin remains, fossilized, forever.
```

## Why this is the right Stage 11

- Reuses fields that already exist in the DNA (no chromosome renumbering, pure
  extension — DNA_LEN 721 → 741, +20 bytes, cheap).
- Makes the FALL itself permissionless and in-circuit (matches the ethos
  already proven in Sentinel's CLTV release and this organism's own
  metabolism apoptosis — nobody should need a secret to prove decay).
- Makes redemption require real proof-of-life (owner must still demonstrate
  control via check-in-style signatures), so it can't be gamed by an outside
  party trying to "rescue" someone else's dying scorpion for their own
  benefit.
- FOSSIL is genuinely terminal — once there, `mode_transition_allowed` simply
  has no entry for `(MODE_FOSSIL, _)`, so no proof can ever move it again.
  That's the actual on-chain grave.
- Fits the existing build pattern exactly: one new `.rs` file
  (`fall_redemption.rs` transition logic in the guest), one new host binary
  (`fall.rs` / `redeem.rs` / `fossilize.rs` or fold into `moult.rs` with a
  `--transition=fall|redeem|fossilize` flag), local proof first, mainnet only
  after Zeku's go-ahead.

## Build order (once you say go)

1. Extend `Dna` struct + `parse_dna`/`serialize_dna` for the 20 new bytes,
   bump `DNA_LEN` to 741 (touches every existing chromosome offset check —
   need to re-verify all 9 chromosomes' existing local proofs still pass
   with the longer blob before touching anything else).
2. Add `MODE_FALLEN`/`MODE_FOSSIL` constants + the 6 new transition-graph
   entries in `mode_transition_allowed`.
3. Write the FALL guest logic: permissionless check against
   `fee_reserve`/`rate` (starvation/cancer) or `deadline_daa`/`grace_blocks`
   (Sentinel-style lapse) — whichever condition is actually true in the old
   DNA, no signature needed.
4. Write the REDEEM guest logic: owner check-in style proof (reuse XMSS
   verify pattern from Chr3), `redemption_progress += 1`.
5. Write the ASCEND guest logic: threshold + deadline check, resupply
   `fee_reserve`, reset fall fields, mode back to VAULT.
6. Write the FOSSILIZE guest logic: permissionless, deadline check, releases
   to `beneficiary_commit`, sets `fossilized=1`, mode=FOSSIL.
7. Local proofs for all 4 paths + negative tests (redeem attempt without
   valid check-in rejected; ascend attempt before threshold rejected;
   fossilize attempt before deadline rejected) — same rigor as every prior
   chromosome.
8. Only after all local proofs pass: wrap into the P2SH covenant pattern,
   confirm reclaim path, confirm fee/mass, THEN ask before any mainnet spend.

Nothing here touches mainnet or spends KAS yet — this is the design. Say the
word and I'll start on step 1.
