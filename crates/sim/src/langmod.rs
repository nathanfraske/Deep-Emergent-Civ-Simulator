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

//! Perceptual geometry: per-modality confusability derived from channel physics (the langmod
//! `perceptual_geometry` consumer, design Part 33.3, record 62.13, the R-LANG-MODALITY seam).
//!
//! The confusability of a modality's feature values is not an authored per-race table and is not a
//! dispersion toward the human auditory target: it is a read-out of the channel physics. A feature
//! value carries a resonator length (a vocal-tract or stopped-pipe geometry); the quarter-wave tube
//! law ([`civsim_physics::laws::tube_resonance`]) maps that length, through the medium's sound speed,
//! to a characteristic-frequency (formant) vector; the frequency-squared absorption law
//! ([`civsim_physics::laws::acoustic_absorption`]) over a typical path (through the existing
//! [`civsim_physics::laws::optical_depth`]) says how much the medium blurs a contrast; and the
//! being's own per-channel resolution (its [`crate::sensorium::Sensorium`] acuity, read as the
//! just-noticeable frequency difference) says how fine a contrast it can hold. Two media, or two
//! vocal geometries, diverge in their confusability geometry from the physics alone, with no
//! `RaceId` and no authored confusability table (Principle 9): the per-race differentiation is the
//! race's own resonator lengths and its own sensorium resolution, which are data, not a table.
//!
//! Everything here is integer fixed-point and draws no randomness (Principle 3): the formant vectors,
//! the pairwise distances, the absorption blur, and the distinguishable-step budget are pure
//! functions of the inputs, walked in a canonical order. The saturation caps and the mode count are
//! caller-supplied engine mechanics, not owner realism values; the channel physics constants (the
//! absorption reference, the sound speed, the resonator lengths) are physics-substrate data.

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::sensorium::{SenseChannelId, Sensorium};

/// Caller-supplied caps and the structural mode count for a perceptual-geometry read-out. These are
/// engine mechanics (the fixed-point saturation caps every physics kernel takes, and how many
/// resonant modes to read per feature value), not owner realism values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceptualParams {
    /// How many resonant modes (formants) to read per feature value (F1, F2, F3 is `modes = 3`).
    /// Clamped to at least one, so a degenerate zero still yields the fundamental.
    pub modes: u32,
    /// The [`civsim_physics::laws::tube_resonance`] frequency cap.
    pub freq_max: Fixed,
    /// The [`civsim_physics::laws::acoustic_absorption`] coefficient cap.
    pub alpha_max: Fixed,
    /// The [`civsim_physics::laws::optical_depth`] cap.
    pub tau_max: Fixed,
    /// The ceiling on a reported confusability score (an identical-formant pair reads this).
    pub confusability_cap: Fixed,
}

/// The derived per-modality perceptual geometry over a set of feature values in one medium: each
/// value's formant-frequency vector, the pairwise confusability table, and the medium-blind
/// distinguishable-step budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerceptualGeometry {
    formants: Vec<Vec<Fixed>>,
    confusability: BTreeMap<(u32, u32), Fixed>,
    contrast_budget: Fixed,
}

impl PerceptualGeometry {
    /// Each feature value's formant-frequency vector, in feature-value order.
    pub fn formants(&self) -> &[Vec<Fixed>] {
        &self.formants
    }

    /// The formant vector of one feature value, or `None` if the index is out of range.
    pub fn formant_vector(&self, i: usize) -> Option<&[Fixed]> {
        self.formants.get(i).map(|v| v.as_slice())
    }

    /// The pairwise confusability table, keyed by the ordered index pair `(i, j)` with `i < j`. A
    /// higher score is more confusable (the absorption-blurred just-noticeable difference covers more
    /// of the two values' formant gap).
    pub fn confusability(&self) -> &BTreeMap<(u32, u32), Fixed> {
        &self.confusability
    }

    /// The confusability of one unordered pair (the index order is normalised to `i < j`), or `None`
    /// for the diagonal or an out-of-range pair.
    pub fn confusability_of(&self, i: usize, j: usize) -> Option<Fixed> {
        if i == j {
            return None;
        }
        let (a, b) = if i < j { (i, j) } else { (j, i) };
        self.confusability.get(&(a as u32, b as u32)).copied()
    }

    /// The distinguishable-step budget: the usable formant span the inventory occupies, measured in
    /// just-noticeable differences. Medium-blind (it reads no absorption), so two media with equal
    /// span-over-just-noticeable-difference have an equal budget even where their confusable pairs
    /// differ (the Steering-Audit equal-capacity invariant: a physics difference does not force a
    /// richness difference).
    pub fn contrast_budget(&self) -> Fixed {
        self.contrast_budget
    }

    /// The pairs at or above a confusability threshold, in canonical `(i, j)` order: the confusable
    /// set. Differs between two media even at an equal contrast budget, because the frequency-squared
    /// absorption blurs high-frequency contrasts more than low-frequency ones.
    pub fn confusable_pairs(&self, threshold: Fixed) -> Vec<(u32, u32)> {
        self.confusability
            .iter()
            .filter(|(_, &c)| c >= threshold)
            .map(|(&k, _)| k)
            .collect()
    }
}

/// Derive the per-modality perceptual geometry of a set of feature values in one medium, reading the
/// channel physics rather than an authored table.
///
/// `lengths` are the per-feature-value resonator lengths (the vocal geometry, per-race data);
/// `sound_speed` is the medium's speed of sound (from [`civsim_physics::laws::speed_of_sound`] over
/// the medium's floor bulk modulus and density); `absorption_reference` is the medium's thermoviscous
/// absorption reference beta; `path` is a typical propagation path; and the being's per-channel
/// resolution is read from its `sensorium` on `channel` as the just-noticeable frequency difference.
///
/// Returns `None` when the sensorium cannot read the channel (a being blind to the channel has no
/// perceptual geometry on it, the sensorium's channel gate) or reads it with a non-positive
/// resolution.
pub fn perceptual_geometry(
    lengths: &[Fixed],
    sound_speed: Fixed,
    absorption_reference: Fixed,
    path: Fixed,
    sensorium: &Sensorium,
    channel: SenseChannelId,
    params: PerceptualParams,
) -> Option<PerceptualGeometry> {
    // The per-channel resolution, read as the just-noticeable frequency difference: a sharper channel
    // carries a smaller value. A channel the sensorium does not read gates perception off entirely.
    let jnd = sensorium.reads(channel)?;
    if jnd <= Fixed::ZERO {
        return None;
    }
    let modes = params.modes.max(1);

    // Each feature value's formant vector, from its resonator length through the tube-resonance law in
    // this medium's sound speed.
    let mut formants: Vec<Vec<Fixed>> = Vec::with_capacity(lengths.len());
    for &l in lengths {
        let mut vector = Vec::with_capacity(modes as usize);
        for n in 1..=modes {
            vector.push(laws::tube_resonance(
                Fixed::from_int(n as i32),
                sound_speed,
                l,
                params.freq_max,
            ));
        }
        formants.push(vector);
    }

    // The usable formant span the inventory occupies, in just-noticeable-difference steps: medium-blind
    // (reads no absorption), so it is the contrast-budget capacity the equal-capacity invariant checks.
    let mut span_lo = params.freq_max;
    let mut span_hi = Fixed::ZERO;
    for vector in &formants {
        for &frequency in vector {
            span_lo = span_lo.min(frequency);
            span_hi = span_hi.max(frequency);
        }
    }
    let usable_span = abs_diff(span_hi, span_lo);
    let contrast_budget = usable_span.checked_div(jnd).unwrap_or(params.freq_max);

    // Pairwise confusability: the just-noticeable difference widened by the medium's absorption blur
    // over the path, against the pair's formant-vector distance. Absorption rises as frequency
    // squared, so a high-frequency pair blurs more, which is what makes two media's confusable sets
    // differ even at an equal contrast budget.
    let mut confusability = BTreeMap::new();
    for i in 0..formants.len() {
        for j in (i + 1)..formants.len() {
            let distance = formant_distance(&formants[i], &formants[j]);
            let mean_freq = pair_mean_frequency(&formants[i], &formants[j]);
            let alpha =
                laws::acoustic_absorption(absorption_reference, mean_freq, params.alpha_max);
            let tau = laws::optical_depth(alpha, path, params.tau_max);
            let widen = Fixed::ONE.saturating_add(tau);
            let effective_jnd = jnd.checked_mul(widen).unwrap_or(params.confusability_cap);
            // How much of the formant gap the blurred just-noticeable difference covers; a zero
            // distance (identical formants) is maximally confusable.
            let conf = match effective_jnd.checked_div(distance) {
                Some(x) => x.min(params.confusability_cap),
                None => params.confusability_cap,
            };
            confusability.insert((i as u32, j as u32), conf);
        }
    }

    Some(PerceptualGeometry {
        formants,
        confusability,
        contrast_budget,
    })
}

/// The L1 distance between two formant vectors over their shared modes, an order-independent
/// saturating sum of the per-mode absolute differences.
fn formant_distance(a: &[Fixed], b: &[Fixed]) -> Fixed {
    let n = a.len().min(b.len());
    Fixed::saturating_sum((0..n).map(|h| abs_diff(a[h], b[h])))
}

/// A representative frequency for a pair: the mean of the two fundamentals (the F1 anchor the
/// absorption weight reads). Zero when either vector is empty.
fn pair_mean_frequency(a: &[Fixed], b: &[Fixed]) -> Fixed {
    match (a.first(), b.first()) {
        (Some(&fa), Some(&fb)) => fa
            .saturating_add(fb)
            .checked_div(Fixed::from_int(2))
            .unwrap_or(Fixed::ZERO),
        _ => Fixed::ZERO,
    }
}

/// Saturating difference in i128, so the subtraction of two bounded frequencies cannot panic or wrap.
fn sat_sub(a: Fixed, b: Fixed) -> Fixed {
    let d = (a.to_bits() as i128) - (b.to_bits() as i128);
    Fixed::from_bits_i128(d).unwrap_or(if d < 0 { Fixed::MIN } else { Fixed::MAX })
}

/// The saturating absolute difference of two values (non-negative, never panicking).
fn abs_diff(a: Fixed, b: Fixed) -> Fixed {
    if a >= b {
        sat_sub(a, b)
    } else {
        sat_sub(b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The acoustic reception channel a sensorium reads for a voice modality (a registry id, not a
    /// closed enum): a test fixture, the real channel set is the physics-channel substrate.
    const HEARING: SenseChannelId = SenseChannelId(1);

    fn params() -> PerceptualParams {
        PerceptualParams {
            modes: 3,
            freq_max: Fixed::from_int(100000),
            alpha_max: Fixed::from_int(10),
            tau_max: Fixed::from_int(100),
            confusability_cap: Fixed::from_int(1000),
        }
    }

    /// A hearing sensorium whose per-channel resolution (the just-noticeable frequency difference) is
    /// `jnd`.
    fn ear(jnd: Fixed) -> Sensorium {
        Sensorium::with([(HEARING, jnd)])
    }

    fn ratio(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// The medium sound speeds from the physics floor, through the resolved speed-of-sound law over
    /// each medium's bulk modulus and density (air ~340 m/s, water ~1480 m/s).
    fn air_speed() -> Fixed {
        laws::speed_of_sound(ratio(142, 1000), ratio(1225, 1000), Fixed::from_int(100000))
    }
    fn water_speed() -> Fixed {
        laws::speed_of_sound(
            Fixed::from_int(2200),
            Fixed::from_int(998),
            Fixed::from_int(100000),
        )
    }

    // Three distinct vocal geometries (resonator lengths, in metres): the per-race producible-sound
    // set, data rather than an authored confusability table.
    fn lengths() -> Vec<Fixed> {
        vec![ratio(15, 100), ratio(17, 100), ratio(20, 100)]
    }

    #[test]
    fn a_blind_channel_has_no_perceptual_geometry() {
        // The sensorium channel gate: a being that does not read the channel derives nothing on it
        // (Principle 9, the sense is a physical-channel reader, not an authored per-race table).
        let deaf = Sensorium::new();
        assert!(
            perceptual_geometry(
                &lengths(),
                air_speed(),
                ratio(1, 100000000),
                Fixed::from_int(10),
                &deaf,
                HEARING,
                params(),
            )
            .is_none(),
            "no channel read, no geometry"
        );
    }

    #[test]
    fn the_read_out_is_deterministic() {
        let run = || {
            perceptual_geometry(
                &lengths(),
                air_speed(),
                ratio(1, 100000000),
                Fixed::from_int(10),
                &ear(Fixed::from_int(50)),
                HEARING,
                params(),
            )
        };
        assert_eq!(run(), run(), "the same inputs replay bit for bit");
    }

    #[test]
    fn the_formants_are_the_tube_resonance_law_over_the_lengths() {
        // The formant vectors are the physics, not an authored table: each equals the tube-resonance
        // law over that length in this medium, and the F1 of a 0.17 m tract in air is the schwa value.
        let geo = perceptual_geometry(
            &lengths(),
            air_speed(),
            ratio(1, 100000000),
            Fixed::from_int(10),
            &ear(Fixed::from_int(50)),
            HEARING,
            params(),
        )
        .unwrap();
        let f1 = geo.formant_vector(1).unwrap()[0].to_f64_lossy();
        assert!(
            (250.0..800.0).contains(&f1),
            "F1 of the 0.17 m tract, got {f1}"
        );
        // A longer resonator sits lower across the board: the 0.20 m tract's F1 is below the 0.15 m.
        let short_f1 = geo.formant_vector(0).unwrap()[0];
        let long_f1 = geo.formant_vector(2).unwrap()[0];
        assert!(long_f1 < short_f1, "a longer resonator resonates lower");
    }

    #[test]
    fn two_media_diverge_in_the_derived_confusability_no_authored_table_no_raceid() {
        // The sim-side non-steering divergence (Principle 9): the SAME lengths and the SAME sensorium
        // in two real media give two different formant sets and two different confusability tables,
        // from the medium's sound speed alone. Nothing here reads a race id; the function takes none.
        let jnd = Fixed::from_int(50);
        let beta = ratio(1, 100000000);
        let path = Fixed::from_int(10);
        let air = perceptual_geometry(
            &lengths(),
            air_speed(),
            beta,
            path,
            &ear(jnd),
            HEARING,
            params(),
        )
        .unwrap();
        let water = perceptual_geometry(
            &lengths(),
            water_speed(),
            beta,
            path,
            &ear(jnd),
            HEARING,
            params(),
        )
        .unwrap();
        assert_ne!(
            air.formants(),
            water.formants(),
            "the same lengths in two media give different formant sets"
        );
        assert_ne!(
            air.confusability(),
            water.confusability(),
            "the derived confusability geometry diverges from the channel physics alone"
        );
    }

    #[test]
    fn equal_capacity_media_share_a_budget_but_differ_in_confusable_pairs() {
        // The Steering-Audit equal-capacity invariant: medium B is medium A with twice the sound speed
        // and twice the just-noticeable difference, so its usable-span-over-just-noticeable-difference
        // (the contrast budget) is equal to A's. Yet its confusable pairs differ, because the
        // frequency-squared absorption blurs B's higher formants more than A's lower ones. A physics
        // difference does not force a richness difference.
        let c_a = Fixed::from_int(343);
        let c_b = Fixed::from_int(686); // exactly twice A's sound speed
        let jnd_a = Fixed::from_int(40);
        let jnd_b = Fixed::from_int(80); // twice A's resolution
        let beta = ratio(1, 100000000);
        let path = Fixed::from_int(10);
        let a = perceptual_geometry(&lengths(), c_a, beta, path, &ear(jnd_a), HEARING, params())
            .unwrap();
        let b = perceptual_geometry(&lengths(), c_b, beta, path, &ear(jnd_b), HEARING, params())
            .unwrap();
        // The contrast budgets are equal to within the fixed-point rounding of the doubled span and
        // resolution (a few units at the last bit).
        let gap = (a.contrast_budget().to_bits() - b.contrast_budget().to_bits()).abs();
        assert!(
            gap <= 4,
            "equal usable-span-over-JND yields an equal contrast budget (gap {gap} bits)"
        );
        // But the confusability geometry differs: the absorption blur is not a pure scaling, so the
        // higher-frequency medium confuses its pairs more.
        assert_ne!(
            a.confusability(),
            b.confusability(),
            "the confusable pairs differ even at an equal budget (frequency-dependent absorption)"
        );
        for (pair, conf_a) in a.confusability() {
            let conf_b = b.confusability().get(pair).copied().unwrap();
            assert!(
                conf_b >= *conf_a,
                "the faster (higher-frequency) medium is at least as confusable, pair {pair:?}"
            );
        }
    }

    #[test]
    fn a_sharper_ear_distinguishes_more_and_a_longer_path_confuses_more() {
        // Two data levers, both physical, both non-steering. A sharper ear (a smaller just-noticeable
        // difference) makes every pair less confusable; a longer propagation path (more absorption
        // blur) makes every pair more confusable. Neither reads a race id.
        let beta = ratio(1, 1000000);
        let sharp = perceptual_geometry(
            &lengths(),
            air_speed(),
            beta,
            Fixed::from_int(10),
            &ear(Fixed::from_int(20)),
            HEARING,
            params(),
        )
        .unwrap();
        let dull = perceptual_geometry(
            &lengths(),
            air_speed(),
            beta,
            Fixed::from_int(10),
            &ear(Fixed::from_int(80)),
            HEARING,
            params(),
        )
        .unwrap();
        for (pair, c_sharp) in sharp.confusability() {
            let c_dull = dull.confusability().get(pair).copied().unwrap();
            assert!(
                c_dull > *c_sharp,
                "a duller ear confuses more, pair {pair:?}"
            );
        }
        let near = perceptual_geometry(
            &lengths(),
            air_speed(),
            beta,
            Fixed::from_int(1),
            &ear(Fixed::from_int(40)),
            HEARING,
            params(),
        )
        .unwrap();
        let far = perceptual_geometry(
            &lengths(),
            air_speed(),
            beta,
            Fixed::from_int(50),
            &ear(Fixed::from_int(40)),
            HEARING,
            params(),
        )
        .unwrap();
        for (pair, c_near) in near.confusability() {
            let c_far = far.confusability().get(pair).copied().unwrap();
            assert!(
                c_far >= *c_near,
                "a longer path confuses more, pair {pair:?}"
            );
        }
    }
}
