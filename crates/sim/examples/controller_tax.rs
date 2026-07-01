// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! The compute tax of the behaviour-controller representation (R-BEHAVIOR-EVOLVE): what it costs to
//! move from the fixed-point reaction norm (hidden width zero) to a small fixed-point recurrent
//! network (hidden width positive), the reserved value `behavior.controller_hidden_width`.
//!
//! Run with `cargo run --release -p civsim-sim --example controller_tax`. This is an observer-side
//! measurement probe, so it uses wall-clock timing (never in canonical state); the controller itself
//! is integer fixed-point and deterministic. It sweeps the hidden width and reports, against the
//! reaction-norm baseline: the heritable weight count, the per-tick evaluation cost (paid by every
//! being every tick), and the per-individual expression cost (paid on birth or promotion). The
//! analytic multiply-accumulate count is printed alongside the measured time so the shape (linear in
//! the weight count for evaluation, quadratic for the current expression path) is visible.

use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::Instant;

use civsim_core::Fixed;
use civsim_sim::controller::{weight_count, Controller, ControllerLayout};
use civsim_sim::evolve::{controller_gene_set, evolve, random_controller_genome, EvolveParams};
use civsim_sim::homeostasis::{AffordanceRegistry, Homeostasis, HomeostaticRegistry, WATER};

/// The per-tick multiply-accumulates (the evaluation cost model, from controller.rs::evaluate).
fn macs(n_in: usize, n_out: usize, h: usize) -> usize {
    if h == 0 {
        n_out * n_in
    } else {
        h * n_in + h * h + n_out * h
    }
}

fn main() {
    let homeo = HomeostaticRegistry::dev_default();
    let afford = AffordanceRegistry::dev_default();
    // Fix the input and output widths at the dev registry (energy, water; move, ingest).
    let base = ControllerLayout::new(&homeo, &afford, 0);
    let (n_in, n_out) = (base.n_in(), base.n_out());

    println!("Compute tax of the controller representation (R-BEHAVIOR-EVOLVE).");
    println!("Dev registry: {n_in} inputs, {n_out} outputs. Baseline is the reaction norm (h = 0).\n");

    // A representative percept and pseudo-weights, held fixed across widths so only the width varies.
    let mut here = BTreeSet::new();
    here.insert(WATER);
    let mut dirs = BTreeMap::new();
    dirs.insert(WATER, (Fixed::from_ratio(3, 5), Fixed::from_ratio(-4, 5)));
    let homeo_state = Homeostasis::new(&homeo, Fixed::from_ratio(1, 2));

    let widths = [0usize, 1, 2, 4, 8, 16, 32, 64];

    println!(
        "{:>5} {:>10} {:>8} {:>12} {:>10} {:>14} {:>10}",
        "h", "weights", "MACs", "eval ns", "eval x", "express ns", "expr x"
    );
    println!("{}", "-".repeat(78));

    let mut base_eval_ns = 0f64;
    let mut base_expr_ns = 0f64;
    let mut checksum: i64 = 0;

    for &h in &widths {
        let wc = weight_count(n_in, n_out, h);
        let layout = ControllerLayout::new(&homeo, &afford, h);
        // Pseudo-weights: a deterministic spread so the accumulation actually mixes terms.
        let weights: Vec<Fixed> = (0..wc)
            .map(|k| Fixed::from_ratio((k as i64 % 7) - 3, 5))
            .collect();
        let controller = Controller::from_weights(n_in, n_out, h, weights);
        let input = layout.build_input(&homeo_state, &here, &dirs);
        let hidden0 = controller.fresh_hidden();

        // --- Per-tick evaluation cost. Bound the total work so wide nets do not run long. ---
        let eval_reps: usize = (200_000_000usize / wc.max(1)).clamp(50_000, 5_000_000);
        let mut hidden = hidden0.clone();
        // Warm up.
        for _ in 0..1000 {
            let (o, nh) = controller.evaluate(&input, &hidden);
            hidden = nh;
            checksum = checksum.wrapping_add(o[0].to_bits());
        }
        let t = Instant::now();
        for _ in 0..eval_reps {
            let (o, nh) = controller.evaluate(black_box(&input), black_box(&hidden));
            hidden = nh;
            checksum = checksum.wrapping_add(o[0].to_bits());
        }
        let eval_ns = t.elapsed().as_nanos() as f64 / eval_reps as f64;

        // --- Per-individual expression cost (once per birth or promotion). O(weight_count^2). ---
        let genes = controller_gene_set(&layout);
        let params = EvolveParams::dev_default();
        let genome = random_controller_genome(&layout, &params, 0xBEEF, 1);
        // express is quadratic in wc; scale reps so total work stays bounded.
        let expr_reps: usize = (400_000_000usize / (wc * wc).max(1)).clamp(3, 5000);
        let t = Instant::now();
        for _ in 0..expr_reps {
            let c = Controller::express(black_box(&genes), black_box(&genome), &layout);
            checksum = checksum.wrapping_add(c.weight(0).to_bits());
        }
        let expr_ns = t.elapsed().as_nanos() as f64 / expr_reps as f64;

        if h == 0 {
            base_eval_ns = eval_ns;
            base_expr_ns = expr_ns;
        }
        println!(
            "{:>5} {:>10} {:>8} {:>12.1} {:>9.1}x {:>14.0} {:>9.1}x",
            h,
            wc,
            macs(n_in, n_out, h),
            eval_ns,
            eval_ns / base_eval_ns,
            expr_ns,
            expr_ns / base_expr_ns,
        );
    }

    println!("\n(black-box checksum {checksum}, ignore)\n");

    // --- World-scale extrapolation of the per-tick evaluation cost. ---
    println!("Per-tick evaluation at world scale (measured eval ns x beings), reaction norm vs a small net:");
    let eval_h0 = time_eval(&homeo, &afford, 0, &homeo_state, &here, &dirs);
    for (label, h) in [("reaction norm h=0", 0usize), ("recurrent h=4", 4), ("recurrent h=8", 8), ("recurrent h=16", 16)] {
        let ns = time_eval(&homeo, &afford, h, &homeo_state, &here, &dirs);
        for &n in &[10_000usize, 100_000, 1_000_000] {
            let ms = ns * n as f64 / 1e6;
            println!("  {label:<18} {n:>9} beings: {ms:>8.2} ms/tick  ({:.1}x baseline)", ns / eval_h0);
        }
    }

    // --- Stage-3 scoring-loop tax: a small end-to-end evolve run, reaction norm vs recurrent. ---
    println!("\nStage-3 scoring loop (evolve, pop 16 x 8 generations x 200-tick episodes):");
    let sparams = EvolveParams {
        pop_size: 16,
        generations: 8,
        ..EvolveParams::dev_default()
    };
    let mut base = 0f64;
    for &h in &[0usize, 1, 2, 4, 8] {
        let layout = ControllerLayout::new(&homeo, &afford, h);
        let t = Instant::now();
        let report = evolve(&layout, &sparams, 0x5EED ^ h as u64);
        let secs = t.elapsed().as_secs_f64();
        if h == 0 {
            base = secs;
        }
        let best = report.best_fitness.last().copied().unwrap_or(0);
        println!(
            "  h = {h:<2}: {secs:>7.3} s  ({:>4.1}x baseline)  best-survival {best}",
            secs / base
        );
    }
}

/// Time a single evaluate call (ns) for a given hidden width, averaged over a bounded rep count.
fn time_eval(
    homeo: &HomeostaticRegistry,
    afford: &AffordanceRegistry,
    h: usize,
    homeo_state: &Homeostasis,
    here: &BTreeSet<civsim_sim::homeostasis::HomeostaticAxisId>,
    dirs: &BTreeMap<civsim_sim::homeostasis::HomeostaticAxisId, (Fixed, Fixed)>,
) -> f64 {
    let layout = ControllerLayout::new(homeo, afford, h);
    let wc = weight_count(layout.n_in(), layout.n_out(), h);
    let weights: Vec<Fixed> = (0..wc).map(|k| Fixed::from_ratio((k as i64 % 7) - 3, 5)).collect();
    let controller = Controller::from_weights(layout.n_in(), layout.n_out(), h, weights);
    let input = layout.build_input(homeo_state, here, dirs);
    let mut hidden = controller.fresh_hidden();
    let reps: usize = (200_000_000usize / wc.max(1)).clamp(50_000, 5_000_000);
    let mut cs = 0i64;
    let t = Instant::now();
    for _ in 0..reps {
        let (o, nh) = controller.evaluate(black_box(&input), black_box(&hidden));
        hidden = nh;
        cs = cs.wrapping_add(o[0].to_bits());
    }
    black_box(cs);
    t.elapsed().as_nanos() as f64 / reps as f64
}
