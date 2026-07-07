# Stage 18 — "A Scorpion Who Generates Worlds"

New chromosome (18): **World Genesis** — transition 15.

## The idea

Reproduction (Chromosome 6, transition 5) spawns more scorpions: same DNA shape,
same rules, a new instance of the same species. Stage 18 is categorically
different — the scorpion authorizes a brand-new covenant genesis output that does
**not** have to be a scorpion at all. Any redeem-script template, any rule set,
any species: a zktimelock, a sentinel, a zkgate, another organism entirely, or
something that has never existed before. The scorpion doesn't reproduce itself
here — it seeds an entire independent world and lets it go free.

What stays permanently, provably verifiable is **provenance**: every world this
scorpion has ever seeded is linked into one forward hash chain
(`world_chain_hash`), the same memory-chain technique as Chromosome 7, but for
children of a different kind. It is append-only, extended by exactly one
transition (15), and it never resets — not even through death and rebirth
(Chromosome 14/17, Stages 14/17). The scorpion's memory of what it created is
eternal, even if the scorpion itself is not.

## New DNA fields (offsets 862..902, DNA_LEN 862 -> 902)

| offset | size | field |
|---|---|---|
| 862 | 32 | `world_chain_hash` — sha256(prev_chain \|\| world_covenant_id \|\| world_script_hash) |
| 894 | 4 | `worlds_spawned` (u32 LE) |
| 898 | 4 | `max_worlds` (u32 LE, birth-time constant, immutable) |

## Invariants

- Global (Phase 1f): `world_chain_hash`, `worlds_spawned` unchanged unless
  `transition == 15`. `max_worlds` is immutable, full stop — checked in every
  transition including 15 itself.
- Transition 15 (`GENERATE_WORLD`):
  - `old.mode == new.mode == VAULT` — only a living, mature scorpion can generate
    a world; the scorpion's own mode is unaffected.
  - `new.worlds_spawned == old.worlds_spawned + 1`
  - `new.worlds_spawned <= old.max_worlds` — the cap, checked BEFORE the witness
    is even read.
  - Witness: `world_covenant_id: [u8;32]`, `world_script_hash: [u8;32]` — read as
    opaque bytes, **zero structural constraints** on what kind of covenant this
    is. That's the whole point.
  - `new.world_chain_hash == sha256(old.world_chain_hash || world_covenant_id || world_script_hash)`

## Proofs (this session, all real RISC0/STARK, `receipt.verify()` passing locally)

1. **HATCH** (transition 0): STEM (gen0) -> VAULT (gen1), `max_worlds=2` set at birth. ✅ verified
2. **GENERATE_WORLD #1** (transition 15): VAULT (gen1) -> VAULT (gen2), `worlds_spawned` 0->1, seeds a
   zktimelock-shaped world. ✅ verified
3. **GENERATE_WORLD #2** (transition 15): VAULT (gen2) -> VAULT (gen3), `worlds_spawned` 1->2 (== `max_worlds`),
   seeds a sentinel-shaped world. ✅ verified
4. **NEGATIVE — GENERATE_WORLD #3**: attempted with `worlds_spawned` going to 3 > `max_worlds=2`.
   Correctly **rejected**: `"cannot exceed max_worlds -- even a god of creation has a limit"`.

Image ID for this guest build: `148b87564d87afc500d5a906f93bb2e4c0af57168ee9fabed7c7b47105059029`
(printed fresh after a forced guest rebuild, per the stale-build-cache lesson from Stage 16).

Status: **local-proof only**, same as Stages 11-17. Not deployed to mainnet.
