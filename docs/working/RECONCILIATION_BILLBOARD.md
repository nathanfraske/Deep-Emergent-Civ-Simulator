# Reconciliation billboard (READ before touching calibration/reserved.toml)

This is the short, always-current coordination pointer for the reserved.toml reconciliation. It is kept
constant and current by the gate. If you edit `calibration/reserved.toml` or add a reserved value, read this
first. Owner directive (2026-07-10): the reconciliation is PRIORITY and must not become a gate; work the
absolute up-to-date version, and no agent pulls stale after it lands.

## Status: IN PROGRESS (Agent C, calibration-reconciliation core)

- **Authoritative base:** current `origin/main` HEAD. ALWAYS rebase onto current `main` before editing
  `reserved.toml` or adding a reserved value. Do not work from a stale copy.
- **The rule (locked, AGENTIC_ADDENDUM.md section 9, the three-way test):** every reserved value is exactly
  one of (1) a fundamental universal constant, (2) a per-world / per-substance datum, or (3) derivable. Anything
  else is a defect. A composite (Stefan-Boltzmann sigma, R, F) DERIVES; it is not authored.
- **Category field:** NOT YET LANDED. Agent C is building a machine-checked `category` field on `ReservedValue`
  plus a CI gate so every entry is born categorized. It is ADDITIVE (an absent field reads UNCLASSIFIED during
  migration). Until it lands, tag any new reserved value in its `basis` with which of the three it is.
- **New reserved values in flight:** #119 (stroke-rate) adds entries; #120 (predation catalog-wound) adds a
  per-covering `fracture_energy`; #121 (R-SOURCE-VECTOR, merged pending) added a `reduction_coefficient` derive
  sentinel. Each must be born categorized once the field lands, and each agent rebases before it edits.

## The sequence

1. Agent C builds the CORE: the three-way census of all 224 entries, the category field + CI gate, the
   fundamentals-home (one closed constants list, composites re-derived). Posts the census for the gate FIRST.
2. Gate confirms the census categorizations.
3. Category field + gate land in a coordinated window (sequenced against #119/#120/#121 so the per-entry sweep
   does not collide).
4. **Each agent (A, B, C) independently blind-bias-checks the reconciliation** with its own section-11-style
   checker, to confirm the categorizations and the fundamentals-home are proper (owner directive).
5. Land on main; rewire any consumer whose value moved category (authored -> derived).
6. This billboard updates to DONE with the authoritative commit, and all branches rebase onto it.

## Out of the core (separate, owner-gated later arcs)

The token/magnitude collapse (per-world magnitudes moving out of the global manifest into each world's own
fixture file) and the fixed-point sub-resolution fundamentals-representation. Flagged, not built in the core.
