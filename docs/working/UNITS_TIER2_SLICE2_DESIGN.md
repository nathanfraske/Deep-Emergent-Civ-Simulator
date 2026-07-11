# Units Tier-2 slice 2: the load-time scale planner (design, for the gate's gate before code)

Slice 2 builds the load-time scale planner and, per the gate's endorsement, converges it with slice 3's floor enforcement on ONE type-level home. This is the design, posted before any slice-2 code, per the per-slice discipline. It authors no mechanism yet. On the gate's gate I build it; consumer lifts (the flagship radiant re-pin) are slice 4+.

## The problem the two seams share

The gate flagged the same root twice. For the PLANNER (slice 2): a live law is hand-written Rust (`laws::radiant_emission` chains `checked_mul`; the contact, coulomb, Reynolds, lever, efficiency laws divide raw), so it exposes NO op graph for the planner to walk and assign scales and widths. For the FLOOR INVARIANT (slice 3): the current near-zero guards are SILENT sentinels (`ln(arg<=0) -> MIN`, `checked_div ... None => substitute`), so the invariant is unbuilt, and a planner that only walks op-graph edges never sees the hand-written divides (including the `evolve.rs` difference-divisor `ONE.div(inv(a)+inv(b)-inv(c))`, invisible to any static site list). Both want the same thing: a law that EXPOSES its structure, so scales DERIVE and every divide and log is covered by construction.

## The type-level home: a law is a declared op graph over scaled quantities

A law is expressed as a `LawExpr`, a small typed expression tree over input quantity handles and the Tier-2 operations (the slice-1 `mul`/`div`/`add`/`sub`/`isqrt` plus `powi` and a rational constant), built ONCE as data. Its structure is therefore inspectable at load: the planner walks it, and the floor check walks it, and the per-tick evaluation walks it under the plan. The MECHANISM (the walk, the planner, the evaluator) is fixed Rust; the law's STRUCTURE (the `LawExpr`) and the quantities' envelopes and floors are DATA. This is the data-op-graph form the gate preferred over hand-threading, and it resolves both seams at once: the planner has a graph to walk, and every `div`/`ln` node is a typed node the floor check cannot miss.

Sketch (illustrative, not final signatures):

```rust
enum LawExpr {
    Input(QuantityId),                       // an input quantity, carrying its declared envelope
    Const(BigRat),                           // a rational law-constant (P9 floor law-content, the value-line's one authored place)
    Mul(Box<LawExpr>, Box<LawExpr>),
    Div { num: Box<LawExpr>, den: Box<LawExpr> },   // den must resolve a declared floor or physical-limit
    Add(Box<LawExpr>, Box<LawExpr>),
    Sub(Box<LawExpr>, Box<LawExpr>),
    Powi(Box<LawExpr>, u32),                  // an integer power, a chain the wide accumulator carries
    Isqrt(Box<LawExpr>),                      // a square root, the quarter-power is two of these
    Ln(Box<LawExpr>),                         // arg must resolve a declared floor or physical-limit
}
```

The flagship radiant emission `sigma * (T_hot^4 - T_cold^4) * emissivity * area` becomes a `LawExpr`: `Mul(Mul(Const(sigma_or_input), Sub(Powi(T_hot, 4), Powi(T_cold, 4))), Mul(emissivity, area))`. The planner walks it, sizes the `Powi(.,4)` chain to i256 (the ~210-bit width the hardware validation measured), assigns each node a scale from `derive_scale_bits`, and the evaluator runs it through the slice-1 `WideAccum` single-round chain. The saturation `None => flux_max` becomes a declared representability cap on the output node, not a silent branch.

## How a law exposes its op graph, and the honest limit

- PREFERRED (data op graph): a law is authored as a `LawExpr`. Its scaling then DERIVES (the planner assigns every edge scale and node width from the declared envelopes), and its divides and logs are covered by construction. This is where every clean arithmetic law goes (the radiant chain, the quarter-power, coulomb `1/separation`, resistance `1/area`, Reynolds `1/viscosity`, the lever ratio, efficiency `net/input`, the `laws.rs` contact `force/den`).
- HAND-THREADED (the named limit): a law that cannot be a clean `LawExpr`, because it needs control flow, a table lookup, or a non-arithmetic step the DSL does not model, stays raw Rust that CALLS the Tier-2 ops with planner-provided scales. Each such law is named as an honest limit with its reason, and it does NOT escape the floor invariant: the type-level `div`/`ln` (below) still requires a declared floor at the call, so a hand-threaded law that divides an un-floored quantity fails loud exactly as a `LawExpr` one would. The type-level divide is the backstop that makes the coverage complete even where the op graph is not exposed.

## The planner: per-law width and scale by interval arithmetic on the log2 bounds

At load, for each law, the planner walks its `LawExpr` bottom-up and assigns every node an exponent interval `[lo_log2, hi_log2]` by interval arithmetic on the inputs' declared envelopes:

- `Input(q)`: the quantity's declared `[lo_log2, hi_log2]`.
- `Const(r)`: the constant's own log2 bracket.
- `Mul(a,b)`: `[lo_a + lo_b, hi_a + hi_b]`.
- `Div{num,den}`: `[lo_num - hi_den, hi_num - lo_den]`, where `lo_den` is bounded by the denominator's declared FLOOR (this is why the floor is required: without it `lo_den` is the storage epsilon and the interval, hence the width, is an observer-dependent artifact, the P10 breach the gate named).
- `Powi(a,n)`: `[n*lo_a, n*hi_a]`.
- `Add/Sub(a,b)`: `[min(lo_a,lo_b) (cancellation floored by the output envelope or a declared floor), max(hi_a,hi_b) + 1]`.
- `Isqrt(a)`: `[floor(lo_a/2), ceil(hi_a/2)]`.

From each node's interval it assigns (i) the intermediate WIDTH: i128 when the interval plus the working scale fits ~120 bits with the guard, i256 when it reaches the ~210-bit chain range the measurement flagged (never i128-by-default, sized per node); and (ii) the node's SCALE via `derive_scale_bits` on its envelope, the `windowed` flag signalling a documented precision window. The output is a fixed per-law `LawPlan` (a scale and a width per node), computed once at load, so the per-tick evaluation is a fixed deterministic integer function. Scales and widths DERIVE from declared bounds; nothing is authored per quantity.

## The floor-or-physical-limit two-declaration scheme (converging slice 3)

The floor invariant is enforced in the same walk. Every `Div` and `Ln` node references an operand quantity (the denominator, the log argument). The planner requires, for that operand, exactly one of two DATA declarations:

- `physical_floor(quantity)`: a per-world reserved-with-basis value below which the quantity is treated as zero or absent, decoupled from the storage epsilon. The interval arithmetic reads this as the operand's `lo` bound.
- `physical_limit_at_zero(law_node)`: a declaration that the zero-boundary is an intentional physical limit (the `contact_pressure` point-load returning `p_max` at zero area), with the limit value it takes there.

A node that declares NEITHER and would silently substitute is the defect: the planner FAILS LOUD at load, the same discipline as an unset reserved value, and names the law and the operand. This covers the hand-written divides and the `evolve.rs` difference-divisor alike (the `Div` node references `recip`, whose interval the planner computes; if `recip` can reach zero, it needs a floor or the law declares the limit). It applies to BOTH twins: `Fixed::div`/`ln` on the CPU path and their CubeCL mirrors in `transcendental.rs` read the SAME declared floor, so they stay bit-identical under it. The floor VALUES are per-world data (reserved-with-basis); the enforcement is the fixed mechanism.

## Byte-neutrality, staging, and discipline

- The slice-2 machinery (the `LawExpr` type, the planner, the floor check, the `LawPlan`) lands BYTE-NEUTRAL: no law is re-expressed yet, so nothing on the run path consumes it and the four pins hold by construction (the same as slice 1).
- Each law LIFT is its own later slice (4+): re-express the law as a `LawExpr`, plan it, evaluate through the slice-1 wide chain. The flagship radiant lift is the intended STATED re-pin (sigma at full precision instead of the ~8-bit Q32.32 truncation), measured and justified per the gate's re-pin discipline.
- Section-9 five-lens runs on each world-content lift, with derive-vs-author and alien-feasibility live (scales, widths, and floors DERIVE from declared envelopes; the floor values are per-world reserved-with-basis, feasible for a world that declares its own bounds). NO frame-blind on the representation and floor change (P9), with the one exception the gate named: if a specific declared floor shapes a CULTURAL or emergent outcome rather than a physical one, I STOP and flag it.

## Open questions for the gate

1. The `LawExpr` DSL scope: the ops above cover the census's idioms (`div`, `ln`, `recip`/`inv` as `Div{Const(1), .}`, `powi` negative as a `Div`, `from_ratio` as a `Const`, `isqrt`). Is there a live law whose shape the DSL cannot express, that must be hand-threaded from the outset (so I name it a limit up front rather than discover it mid-lift)?
2. The floor declarations' home: `physical_floor` as a per-quantity field on the quantity registry (reserved-with-basis), and `physical_limit_at_zero` as a per-law-node declaration on the `LawExpr`. Confirm both live as data, not code constants.
3. The hand-threaded backstop: is the type-level `Fixed::div`/`ln` requiring a declared-floor operand (the complete coverage) in scope for slice 3, or does slice 2 land the planner and `LawExpr` first and slice 3 add the type-level `Fixed` divide after? I lean landing the planner + `LawExpr` + the `LawExpr`-node floor check in slice 2, and the type-level `Fixed::div`/`ln` backstop (covering the still-hand-threaded laws) as slice 3, so each is a reviewable step.

On the gate's gate of this design I build slice 2 (the machinery, byte-neutral), and post it. Off current `main` (I forward-merge #130 before the arc merge, not urgent while building).
