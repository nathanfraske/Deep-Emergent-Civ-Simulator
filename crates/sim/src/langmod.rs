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
//! being's own per-channel resolution (its [`crate::sensorium::Sensorium`] resolution, the
//! just-noticeable frequency difference, distinct from the `[0, ONE]` acuity gate after the
//! R-SENSORIUM split) says how fine a contrast it can hold. Two media, or two vocal geometries,
//! diverge in their confusability geometry from the physics alone, with no `RaceId` and no authored
//! confusability table (Principle 9): the per-race differentiation is the race's own resonator
//! lengths and its own sensorium resolution, which are data, not a table.
//!
//! Everything here is integer fixed-point and draws no randomness (Principle 3): the formant vectors,
//! the pairwise distances, the absorption blur, and the distinguishable-step budget are pure
//! functions of the inputs, walked in a canonical order. The saturation caps and the mode count are
//! caller-supplied engine mechanics, not owner realism values; the channel physics constants (the
//! absorption reference, the sound speed, the resonator lengths) are physics-substrate data.
//!
//! This module also carries the per-being language capability gate (Part 33.3, record 62.13, the
//! resolved R-LANG-MODALITY work): a communication channel is usable to a being only to the extent
//! it can both produce and perceive on it, the Liebig limiting half of the two. The production half
//! is [`Body::function_integrity`] of the articulating function, kneed at the wound function-loss
//! threshold; the perception half is the being's [`Sensorium`] acuity for the reception channel. No
//! race identity enters: two races diverge only through their body integrity and sensorium acuity
//! data, run through one kernel (Principle 9), and the gate is a pure float-free function of the
//! body, the sensorium, and the reserved threshold (Principle 3).

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::body::{Body, FunctionId};
use crate::language::{
    FeatureDimId, FeatureValueId, FormSegment, FormSystem, ProductionModalityId,
};
use crate::race::Articulation;
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
/// Returns `None` when the sensorium has no resolution set for the channel (a being that cannot
/// discriminate on the channel has no perceptual geometry on it, the sensorium's resolution gate) or
/// carries a non-positive resolution. The resolution is the discrimination side of the sensorium,
/// distinct from the `[0, ONE]` acuity gate the perception beat reads (the R-SENSORIUM split).
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
    // carries a smaller value. Read from the sensorium's RESOLUTION map, distinct from the `[0, ONE]`
    // acuity gate the perception beat and capability_halves read (the R-SENSORIUM acuity/resolution
    // split): a channel with no resolution set gates the geometry off entirely.
    let jnd = sensorium.resolution(channel)?;
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

/// Derive a race's perceptual geometry from the SHARED base sound geometry and its OWN articulation
/// data (design Part 33.3, the per-race application of [`perceptual_geometry`]). It scales the base
/// resonator lengths by the race's vocal-tract scale (a larger tract lengthens the resonators and
/// lowers the formants, through the tube-resonance law) and reads its hearing resolution as the
/// sensorium's just-noticeable difference, then runs the shared kernel over that per-race geometry.
///
/// Two races diverge from their articulation data alone (Principle 9): the kernel reads no `RaceId`
/// and no per-race table, only the two scalars an [`Articulation`] carries, so a big-tracted race and
/// a small-tracted one, or a sharp-eared and a dull-eared one, fall out of the same code on different
/// data. Pure fixed-point and deterministic (Principle 3): one multiply per base length and the
/// shared kernel, no float and no RNG. Returns `None` on the shared kernel's terms (an empty base
/// geometry, or a non-positive resolution).
pub fn articulated_geometry(
    base_lengths: &[Fixed],
    sound_speed: Fixed,
    absorption_reference: Fixed,
    path: Fixed,
    articulation: &Articulation,
    channel: SenseChannelId,
    params: PerceptualParams,
) -> Option<PerceptualGeometry> {
    let scaled: Vec<Fixed> = base_lengths
        .iter()
        .map(|&l| l.mul(articulation.vocal_tract_scale))
        .collect();
    let sensorium = Sensorium::with_resolution([(channel, articulation.hearing_resolution)]);
    perceptual_geometry(
        &scaled,
        sound_speed,
        absorption_reference,
        path,
        &sensorium,
        channel,
        params,
    )
}

/// The two Liebig halves of a being's capability on one communication channel, each in `[0, ONE]`.
/// Both are kept (rather than only their minimum) so the sibling acquisition split can subtract the
/// production half from the perception half without recomputing either. [`CapabilityGate::gate`] is
/// the usable channel capability, the limiting half.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CapabilityGate {
    /// The reception half: the sensorium's acuity for the channel, zero if it cannot read it.
    pub perception: Fixed,
    /// The production half: [`Body::function_integrity`] of the articulating function.
    pub production: Fixed,
}

impl CapabilityGate {
    /// The usable channel capability: the Liebig minimum of the two halves. A voice with no ear, or
    /// an ear with no voice, gates the channel shut.
    pub fn gate(self) -> Fixed {
        if self.perception < self.production {
            self.perception
        } else {
            self.production
        }
    }
}

/// Both halves of a being's capability on a channel: the production side from the body's function
/// integrity (kneed at `function_loss_threshold`), the perception side from the sensorium's acuity
/// for the reception `channel`. A pure, deterministic function of the two data inputs and the
/// reserved threshold; no race identity, no float.
///
/// The `function_loss_threshold` is RESERVED (`body.function_loss_threshold` in the calibration
/// manifest), the knee point [`Body::function_integrity`] applies. Basis: the wound model's
/// function-loss threshold, set equal to it so the language capability-gate floor and the wound
/// model agree (record 62.13, "the floor set equal to the wound model's function-loss threshold for
/// consistency").
pub fn capability_halves(
    body: &Body,
    produce_function: FunctionId,
    sensorium: &Sensorium,
    channel: SenseChannelId,
    function_loss_threshold: Fixed,
) -> CapabilityGate {
    let production = body.function_integrity(produce_function, function_loss_threshold);
    // The being reads the channel with this acuity, or not at all (a channel it cannot perceive
    // gates the capability to zero regardless of production). The acuity is a [0, ONE] capability,
    // so it is clamped to that range: the Liebig gate and the acquisition split both assume it, and
    // an out-of-range sensorium datum must not push the split past its documented bound.
    let perception = sensorium
        .reads(channel)
        .unwrap_or(Fixed::ZERO)
        .clamp(Fixed::ZERO, Fixed::ONE);
    CapabilityGate {
        perception,
        production,
    }
}

/// The usable capability of a being on one communication channel: the Liebig minimum of its
/// perception of the channel and its production on it (design Part 33.3). The perception half is the
/// sensorium's acuity for `channel`; the production half is the body's integrity on
/// `produce_function`, kneed at the reserved wound function-loss threshold. Returns `[0, ONE]`. Use
/// [`capability_halves`] when both halves are needed (the acquisition split). Deterministic,
/// float-free, and free of any race branch.
pub fn capability_gate(
    body: &Body,
    produce_function: FunctionId,
    sensorium: &Sensorium,
    channel: SenseChannelId,
    function_loss_threshold: Fixed,
) -> Fixed {
    capability_halves(
        body,
        produce_function,
        sensorium,
        channel,
        function_loss_threshold,
    )
    .gate()
}

/// The acquisition split of a being on one communication channel: its perceive capability minus its
/// produce capability, in `[-ONE, ONE]` (design Part 33.6, the R-LANG-MODALITY acquisition work).
/// The receptive-bilingual asymmetry, where a learner understands more of a language than it can
/// produce, is not an authored bias: it falls straight out of the gap between the two capability
/// halves [`capability_halves`] already computes. A positive split (perception outstrips production)
/// is the receptive learner; a negative split the reverse; a zero split a channel matched in both
/// halves. The mechanism reads the same body, sensorium, and reserved threshold as the capability
/// gate, so two races diverge only through their body integrity and sensorium acuity data run
/// through one kernel, never a race branch (Principle 9), and it is a pure, float-free, saturating
/// function that replays bit for bit (Principle 3).
pub fn acquisition_split(
    body: &Body,
    produce_function: FunctionId,
    sensorium: &Sensorium,
    channel: SenseChannelId,
    function_loss_threshold: Fixed,
) -> Fixed {
    let halves = capability_halves(
        body,
        produce_function,
        sensorium,
        channel,
        function_loss_threshold,
    );
    sat_sub(halves.perception, halves.production)
}

/// A dispersion prior over a race's own producible-sound geometry: for each feature value, a
/// spread-maximising weight that is higher where the value is more distinguishable from the rest of
/// the inventory, masked by the being's capability on that value (design Part 33.3, the
/// R-LANG-MODALITY / R-LANG-DET phoneme-prior seam). This replaces an authored phoneme inventory:
/// UPSID and PHOIBLE describe the human vocal tract's confusability geometry, so they become the
/// human race's data row (its resonator lengths and sensorium acuity, read through
/// [`perceptual_geometry`]) rather than the global prior every race is pulled toward. A value's
/// distinctiveness is the sum over the other values of `ONE / (ONE + confusability)`, so a value
/// well separated from the rest scores high and a value confusable with many scores low; the
/// per-value capability `gate` then masks it, so a value the race cannot produce or perceive (gate
/// zero) gets zero prior, and a partly-producible value is scaled down. No race id enters and no
/// human inventory is authored (Principle 9): two races with different confusability geometry (from
/// different media or vocal geometries, through [`perceptual_geometry`]) get different priors from
/// the one kernel.
///
/// The prior is returned in feature-value order, the same order the geometry was built from its
/// values, each paired with [`FeatureValueId`] carrying that index; a caller with arbitrary value
/// ids zips the returned priors with its own value list, which the geometry was built in step with.
/// `gate` shorter than the value count masks the missing tail to zero (an unrated value is not
/// producible). The per-value gate vector is the caller's per-value producibility, built at wire
/// time from the race's producible feature set; [`capability_gate`] itself resolves one channel-wide
/// capability, so the caller broadcasts or refines it per value. Pure fixed-point over a canonical
/// walk, so it replays bit for bit.
pub fn phoneme_priors(geo: &PerceptualGeometry, gate: &[Fixed]) -> Vec<(FeatureValueId, Fixed)> {
    let n = geo.formants().len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // The value's total distinctiveness: how separable it is from every other value, summed as
        // ONE / (ONE + confusability). A distinct pair (low confusability) contributes near ONE, a
        // confusable pair (high confusability) contributes near zero.
        let distinctiveness = Fixed::saturating_sum((0..n).filter(|&j| j != i).map(|j| {
            let conf = geo.confusability_of(i, j).unwrap_or(Fixed::ZERO);
            Fixed::ONE
                .checked_div(Fixed::ONE.saturating_add(conf))
                .unwrap_or(Fixed::ZERO)
        }));
        // The capability mask: a value the race cannot produce or perceive (gate zero) is masked to
        // zero prior; a partly-producible value scales its dispersion by its capability.
        let cap = gate.get(i).copied().unwrap_or(Fixed::ZERO);
        let prior = if cap <= Fixed::ZERO {
            Fixed::ZERO
        } else {
            distinctiveness.checked_mul(cap).unwrap_or(Fixed::ZERO)
        };
        out.push((FeatureValueId(i as u32), prior));
    }
    out
}

/// Select the producible feature values from a set of phoneme priors and a producibility threshold
/// (design Part 33.3): a value enters the producible inventory only if its dispersion-and-capability
/// prior reaches the threshold, so a value the race cannot reliably produce or perceive is left out.
/// A gate-masked value (zero prior) is below any positive threshold, and a low-dispersion value
/// (highly confusable with the rest of the inventory) may also fall below it. The result is in
/// ascending [`FeatureValueId`] order (canonical) and deduplicated, so the selection is deterministic
/// regardless of the order the priors arrive in (Principle 3), and it reads no race id (Principle 9):
/// two races select different inventories only because their priors differ. The threshold is reserved
/// owner data (`articulation.producibility_threshold`), never fabricated.
pub fn producible_values(
    priors: &[(FeatureValueId, Fixed)],
    threshold: Fixed,
) -> Vec<FeatureValueId> {
    let mut out: Vec<FeatureValueId> = priors
        .iter()
        // A gate-masked value (zero prior) never enters, even at a zero threshold: it cannot be
        // produced or perceived at all, so it is not a candidate regardless of the threshold.
        .filter(|(_, prior)| *prior > Fixed::ZERO && *prior >= threshold)
        .map(|(id, _)| *id)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The reason a phonetic form system could not be derived, kept distinct from a silent empty system
/// so the build fails loud (Principle 11) rather than coining empty words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FormSystemError {
    /// No feature value was producible: the race is blind to the channel (no perceptual geometry) or
    /// the producibility threshold excluded every candidate. A form system with an empty inventory
    /// would coin only empty words, so the build refuses rather than fabricating a silent language.
    EmptyInventory,
}

/// Bridge a set of producible feature values to a [`FormSystem`] (design Part 33.3, the seam that was
/// missing between [`phoneme_priors`] and a coinable inventory): each producible value becomes a
/// one-feature [`FormSegment`] on its dimension, and the segments become the coining inventory of a
/// form system in the given modality with the given word-length range. Fails loud on an empty
/// inventory rather than building a form system that coins only empty words. Deterministic: the values
/// are taken in the order given (the canonical [`FeatureValueId`] order [`producible_values`]
/// produces), so the inventory is a pure function of the selection.
pub fn form_system_from_values(
    modality: ProductionModalityId,
    dim: FeatureDimId,
    values: &[FeatureValueId],
    min_len: u32,
    max_len: u32,
) -> Result<FormSystem, FormSystemError> {
    if values.is_empty() {
        return Err(FormSystemError::EmptyInventory);
    }
    let inventory: Vec<FormSegment> = values
        .iter()
        .map(|&v| FormSegment::new([(dim, v)]))
        .collect();
    Ok(FormSystem::new(modality, inventory, min_len, max_len))
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
    /// `jnd`. The value is a RESOLUTION, not an acuity, so it is set on the resolution map that
    /// perceptual_geometry reads (the R-SENSORIUM split).
    fn ear(jnd: Fixed) -> Sensorium {
        Sensorium::with_resolution([(HEARING, jnd)])
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
    fn two_races_diverge_in_perceptual_geometry_from_their_articulation_alone_no_raceid() {
        use crate::race::Articulation;
        // Per-race phonetics from data (Part 33.3, Principle 9): two races share the base sound
        // geometry and the medium, differing only in their Articulation. articulated_geometry reads
        // the two scalars an Articulation carries, no RaceId and no per-race table, so the divergence
        // falls out of the same kernel on different data.
        let base = lengths();
        let c = air_speed();
        let beta = ratio(1, 100000000);
        let path = Fixed::from_int(10);
        let geo = |a: &Articulation| {
            articulated_geometry(&base, c, beta, path, a, HEARING, params()).unwrap()
        };

        // A longer vocal tract lengthens the resonators and lowers the formants (tube resonance,
        // frequency proportional to speed over length): a full-scale tract's F1 is below a
        // half-scale tract's, holding the ear equal.
        let jnd = Fixed::from_int(50);
        let long_tract = Articulation {
            vocal_tract_scale: Fixed::ONE,
            hearing_resolution: jnd,
        };
        let short_tract = Articulation {
            vocal_tract_scale: Fixed::from_ratio(1, 2),
            hearing_resolution: jnd,
        };
        let f1 = |g: &PerceptualGeometry| g.formant_vector(0).unwrap()[0];
        assert!(
            f1(&geo(&long_tract)) < f1(&geo(&short_tract)),
            "a longer tract lowers the formants: {:?} < {:?}",
            f1(&geo(&long_tract)),
            f1(&geo(&short_tract))
        );

        // A sharper ear (a smaller just-noticeable difference) discriminates more, widening the
        // contrast budget, holding the vocal tract equal.
        let sharp_ear = Articulation {
            vocal_tract_scale: Fixed::ONE,
            hearing_resolution: Fixed::from_int(20),
        };
        let dull_ear = Articulation {
            vocal_tract_scale: Fixed::ONE,
            hearing_resolution: Fixed::from_int(80),
        };
        assert!(
            geo(&sharp_ear).contrast_budget() > geo(&dull_ear).contrast_budget(),
            "a sharper ear widens the contrast budget"
        );

        // No RaceId, and deterministic: identical articulation data reads bit-identically.
        assert_eq!(
            geo(&long_tract),
            geo(&long_tract),
            "identical articulation data replays bit for bit"
        );
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

    #[test]
    fn phoneme_priors_differ_by_geometry_and_mask_below_the_gate_no_raceid() {
        // The dispersion prior derives from the race's OWN confusability geometry, not an authored
        // human inventory. The same lengths in two media (air, water) give two confusability
        // geometries, so two different phoneme priors, from the channel physics alone. The function
        // reads no race id and no phoneme table.
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
        let n = lengths().len();
        let full_gate = vec![Fixed::ONE; n];
        let pa = phoneme_priors(&air, &full_gate);
        let pw = phoneme_priors(&water, &full_gate);
        assert_eq!(pa.len(), n, "one prior per feature value");
        assert_ne!(
            pa, pw,
            "two media give two different phoneme priors (no authored inventory, no RaceId)"
        );
        // Returned in feature-value order, each a non-negative producible prior.
        for (idx, (id, prior)) in pa.iter().enumerate() {
            assert_eq!(id.0 as usize, idx, "priors returned in feature-value order");
            assert!(*prior > Fixed::ZERO, "a producible value carries a prior");
        }
        // Masking: a value the race cannot produce or perceive (gate zero) gets zero prior; the
        // producible siblings keep theirs.
        let mut masked = vec![Fixed::ONE; n];
        masked[1] = Fixed::ZERO;
        let pm = phoneme_priors(&air, &masked);
        assert_eq!(
            pm[1].1,
            Fixed::ZERO,
            "a value below the gate gets zero prior"
        );
        assert!(
            pm[0].1 > Fixed::ZERO && pm[2].1 > Fixed::ZERO,
            "the producible values keep a prior"
        );
        // A partly-producible value scales its prior down (mask is multiplicative, not just binary).
        let mut half = vec![Fixed::ONE; n];
        half[0] = Fixed::from_ratio(1, 2);
        let ph = phoneme_priors(&air, &half);
        assert!(
            ph[0].1 < pa[0].1 && ph[0].1 > Fixed::ZERO,
            "a half-producible value keeps a reduced, positive prior"
        );
        // Deterministic replay.
        assert_eq!(phoneme_priors(&air, &full_gate), pa);
    }

    #[test]
    fn producible_values_select_above_the_threshold_in_canonical_order() {
        // The producibility gate (Part 33.3): only values whose prior reaches the threshold enter the
        // inventory, folded in ascending FeatureValueId order so the selection is deterministic
        // regardless of input order. A gate-masked (zero-prior) value never enters, even at a zero
        // threshold.
        let priors = vec![
            (FeatureValueId(2), Fixed::from_ratio(3, 4)),  // above
            (FeatureValueId(0), Fixed::from_ratio(1, 10)), // below
            (FeatureValueId(1), Fixed::from_ratio(1, 2)),  // above
            (FeatureValueId(3), Fixed::ZERO),              // masked
        ];
        let threshold = Fixed::from_ratio(1, 4);
        assert_eq!(
            producible_values(&priors, threshold),
            vec![FeatureValueId(1), FeatureValueId(2)],
            "the above-threshold values, in ascending id order, masked and below-threshold left out"
        );
        // A masked (zero-prior) value stays out even when the threshold is zero.
        assert_eq!(
            producible_values(&priors, Fixed::ZERO),
            vec![FeatureValueId(0), FeatureValueId(1), FeatureValueId(2)],
            "at a zero threshold every positive-prior value enters, but the masked one does not"
        );
        // A threshold above every prior yields an empty inventory (the fail-loud-on-empty case 2d guards).
        assert!(producible_values(&priors, Fixed::from_int(2)).is_empty());
    }

    #[test]
    fn form_system_from_values_bridges_an_inventory_and_fails_loud_on_empty() {
        use crate::language::{FeatureDimId, ProductionModalityId};
        // The bridge: producible values become one-feature form segments, and the segments become a
        // coinable form system. An empty inventory fails loud rather than coining empty words.
        let fs = form_system_from_values(
            ProductionModalityId(0),
            FeatureDimId(0),
            &[FeatureValueId(0), FeatureValueId(2)],
            1,
            2,
        )
        .expect("a non-empty inventory builds a form system");
        assert_eq!(fs.inventory().len(), 2, "one segment per producible value");
        assert!(!fs.is_empty());
        assert!(
            matches!(
                form_system_from_values(ProductionModalityId(0), FeatureDimId(0), &[], 1, 2),
                Err(FormSystemError::EmptyInventory)
            ),
            "an empty inventory fails loud, never coins empty words"
        );
    }

    #[test]
    fn the_full_phonetic_pipeline_composes_and_a_sharper_ear_yields_a_richer_inventory() {
        use crate::language::{FeatureDimId, ProductionModalityId};
        use crate::race::Articulation;
        // The whole 2b-2c-2d pipeline composes: base geometry -> articulated_geometry -> phoneme
        // priors (broadcast capability gate) -> producible_values (threshold) -> form_system_from_values.
        // A sharper ear (smaller just-noticeable difference) discriminates more sounds, so more values
        // clear the threshold and its inventory is at least as rich, all from the articulation data
        // through one pipeline with no race id.
        let base: Vec<Fixed> = (10..=16).map(|cm| ratio(cm, 100)).collect(); // seven candidate sounds
        let c = air_speed();
        let beta = ratio(1, 100000000);
        let path = Fixed::from_int(10);
        // A threshold between the two ears' prior ranges: a sharp ear's priors all clear it, a dull
        // ear's all fall below it (a labelled fixture, tuned to the physics so the divergence shows).
        let threshold = Fixed::from_ratio(57, 10);

        let inventory_size = |hearing_resolution: Fixed| -> Result<usize, FormSystemError> {
            let art = Articulation {
                vocal_tract_scale: Fixed::ONE,
                hearing_resolution,
            };
            let geo = articulated_geometry(&base, c, beta, path, &art, HEARING, params()).unwrap();
            let gate = vec![Fixed::ONE; base.len()]; // full channel capability, broadcast per value
            let priors = phoneme_priors(&geo, &gate);
            let values = producible_values(&priors, threshold);
            form_system_from_values(ProductionModalityId(0), FeatureDimId(0), &values, 1, 3)
                .map(|fs| fs.inventory().len())
        };

        // The sharp ear discriminates every candidate, so all seven clear the threshold and build a
        // full inventory; the dull ear confuses them, so none clear it and the build fails loud rather
        // than coining empty words. The richer inventory falls out of the hearing resolution alone.
        let sharp = inventory_size(Fixed::from_int(15)).expect("a sharp ear builds an inventory");
        assert_eq!(
            sharp,
            base.len(),
            "the sharp ear produces every candidate sound"
        );
        let dull = inventory_size(Fixed::from_int(120));
        assert!(
            matches!(dull, Err(FormSystemError::EmptyInventory)),
            "the dull ear clears nothing and fails loud: {dull:?}"
        );
        let dull_size = dull.unwrap_or(0);
        assert!(
            sharp > dull_size,
            "a sharper ear yields a strictly richer producible inventory: {sharp} > {dull_size}"
        );
    }
}

#[cfg(test)]
mod capability_gate_tests {
    use super::*;
    use crate::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
    use crate::body::{BodyParams, BLOOD, F_VITAL_CORE};

    // FIXTURE values, never read from the manifest.
    const VOICE: SenseChannelId = SenseChannelId(1); // the reception channel a voice fills (hearing)
    const SCENT: SenseChannelId = SenseChannelId(2);
    const LOSS_THRESHOLD: Fixed = Fixed::from_bits(1 << 31); // 0.5, a FIXTURE function-loss knee

    fn plan(mass: (i64, i64), legs: usize) -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(mass.0, mass.1),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            }],
            locomotion: (0..legs).map(|_| 1u16).collect(),
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(1, 2),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        }
    }

    fn body() -> Body {
        Body::from_body_plan(
            &plan((3, 4), 4),
            BLOOD,
            &BodyParams::dev_default(),
            &BodyPlanRegistry::dev_default(),
        )
    }

    #[test]
    fn two_races_with_identical_integrity_and_acuity_gate_bit_identically() {
        // No RaceId enters the kernel: two races that happen to share body integrity and sensorium
        // acuity run through the one function and land on the same bits (Principle 9, Principle 3).
        let race_a = body();
        let race_b = body();
        let sens = Sensorium::with([(VOICE, Fixed::from_ratio(3, 4))]);
        let ga = capability_gate(&race_a, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        let gb = capability_gate(&race_b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert_eq!(
            ga.to_bits(),
            gb.to_bits(),
            "identical data gates to identical bits"
        );
        // And it is the Liebig minimum of the two halves.
        let halves = capability_halves(&race_a, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert_eq!(
            ga,
            halves.perception.min(halves.production),
            "the gate is the limiting half"
        );
        assert_eq!(
            ga,
            Fixed::from_ratio(3, 4),
            "here perception limits: an intact voice, a keen-but-imperfect ear"
        );
    }

    #[test]
    fn one_sensorium_feeds_acuity_to_the_gate_and_resolution_to_the_geometry() {
        // The R-SENSORIUM acuity/resolution split (WP5) end to end: a single sensorium carries a full
        // acuity gate AND a Hz-scale just-noticeable difference on the same voice channel.
        // capability_halves reads the acuity, perceptual_geometry reads the resolution, and neither
        // reads the other's quantity, so the value that is a valid acuity (one) does not corrupt the
        // geometry as an implausible one-hertz JND, and the 50 Hz resolution does not clamp the gate.
        let jnd = Fixed::from_int(50); // 50 Hz: a plausible JND, an implausible acuity
        let sens = Sensorium::with([(VOICE, Fixed::ONE)]).and_resolution([(VOICE, jnd)]);

        // The production-perception half reads the acuity gate (one), not the 50 Hz resolution.
        let halves = capability_halves(&body(), F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert_eq!(
            halves.perception,
            Fixed::ONE,
            "capability reads the acuity gate, not the JND"
        );

        // The perceptual geometry reads the resolution (50 Hz), and gates on the resolution field: an
        // acuity-only sensorium (no resolution) yields no geometry, proving the geometry does not read
        // the acuity map.
        let lengths = [
            Fixed::from_ratio(17, 100),
            Fixed::from_ratio(15, 100),
            Fixed::from_ratio(13, 100),
        ];
        let sound_speed = laws::speed_of_sound(
            Fixed::from_ratio(142, 1000),
            Fixed::from_ratio(1225, 1000),
            Fixed::from_int(100000),
        );
        let params = PerceptualParams {
            modes: 3,
            freq_max: Fixed::from_int(100000),
            alpha_max: Fixed::from_int(10),
            tau_max: Fixed::from_int(100),
            confusability_cap: Fixed::from_int(1000),
        };
        let beta = Fixed::from_ratio(1, 100000000);
        let path = Fixed::from_int(10);
        assert!(
            perceptual_geometry(&lengths, sound_speed, beta, path, &sens, VOICE, params).is_some(),
            "the resolution feeds a perceptual geometry"
        );
        let acuity_only = Sensorium::with([(VOICE, Fixed::ONE)]);
        assert!(
            perceptual_geometry(&lengths, sound_speed, beta, path, &acuity_only, VOICE, params)
                .is_none(),
            "an acuity gate with no resolution yields no geometry: the geometry reads the resolution"
        );
    }

    #[test]
    fn a_wound_below_the_threshold_drops_production_to_the_floor() {
        let mut b = body();
        let sens = Sensorium::with([(VOICE, Fixed::ONE)]);
        // Intact: production is full, so the gate is open (perception is full here too).
        let before = capability_gate(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert_eq!(before, Fixed::ONE, "an intact being gates fully open");
        // Wound the articulating part below the loss threshold: production floors, and the Liebig
        // minimum drags the whole gate to zero even though perception is untouched.
        let torso = b.parts.iter().position(|p| p.name == "torso").unwrap();
        b.parts[torso].condition.integrity = Fixed::from_ratio(1, 4); // below the 0.5 threshold
        let halves = capability_halves(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert_eq!(
            halves.production,
            Fixed::ZERO,
            "a wound past the function-loss threshold floors production"
        );
        assert_eq!(
            halves.perception,
            Fixed::ONE,
            "perception is unaffected by the production-side wound"
        );
        assert_eq!(
            capability_gate(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD),
            Fixed::ZERO,
            "the limiting production half gates the channel shut"
        );
    }

    #[test]
    fn an_unread_channel_gates_to_zero() {
        let b = body();
        // The being reads scent well but does not read the voice channel at all.
        let sens = Sensorium::with([(SCENT, Fixed::ONE)]);
        let halves = capability_halves(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD);
        assert!(
            halves.production > Fixed::ZERO,
            "production on the channel is available"
        );
        assert_eq!(
            halves.perception,
            Fixed::ZERO,
            "but the channel is unread: perception is zero"
        );
        assert_eq!(
            capability_gate(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD),
            Fixed::ZERO,
            "a channel the being cannot perceive gates shut regardless of production"
        );
    }

    #[test]
    fn a_manual_channel_degrades_gracefully_as_limbs_are_lost() {
        // A signed channel produced through the DERIVED limbs (emergent-anatomy step one): a limb is a
        // limb by its physics, so losing one of four weakens the manual production (the mean of the limb
        // bearers) without erasing it, and perception can still be the limiter. The manual channel's gate
        // is the Liebig minimum of perception and the derived limb integrity.
        let mut b = body();
        let sens = Sensorium::with([(VOICE, Fixed::ONE)]);
        let gate = |body: &Body| {
            CapabilityGate {
                perception: sens.reads(VOICE).unwrap_or(Fixed::ZERO),
                production: body.locomotor_integrity(LOSS_THRESHOLD),
            }
            .gate()
        };
        assert_eq!(gate(&b), Fixed::ONE, "four intact limbs sign at full");
        let limb = b
            .parts
            .iter()
            .position(|p| p.name.starts_with("limb"))
            .unwrap();
        b.parts[limb].condition.severed = true;
        let degraded = gate(&b);
        assert!(
            degraded > Fixed::ZERO && degraded < Fixed::ONE,
            "losing one limb weakens the manual channel without silencing it ({degraded:?})"
        );
    }

    #[test]
    fn the_gate_replays_bit_identically() {
        let run = || {
            let b = body();
            let sens = Sensorium::with([(VOICE, Fixed::from_ratio(5, 8))]);
            capability_gate(&b, F_VITAL_CORE, &sens, VOICE, LOSS_THRESHOLD).to_bits()
        };
        assert_eq!(run(), run(), "the same inputs gate to the same bits");
    }

    #[test]
    fn the_acquisition_split_is_perceive_minus_produce_and_mirrors_on_swap_no_raceid() {
        // A being strong in perception, weak in production: a full ear, a voice weakened by a wound to
        // the vital-core articulator (production degraded below full, perception intact). The split is the
        // perceive minus produce gap, positive: the receptive-bilingual asymmetry falls out of the gap,
        // not an authored bias. The function reads no race id.
        let mut perceptive = body();
        let torso = perceptive
            .parts
            .iter()
            .position(|p| p.name == "torso")
            .unwrap();
        perceptive.parts[torso].condition.integrity = Fixed::from_ratio(3, 4);
        let full_ear = Sensorium::with([(VOICE, Fixed::ONE)]);
        let halves = capability_halves(&perceptive, F_VITAL_CORE, &full_ear, VOICE, LOSS_THRESHOLD);
        assert!(
            halves.perception > halves.production,
            "a keen ear, a weakened voice"
        );
        let split = acquisition_split(&perceptive, F_VITAL_CORE, &full_ear, VOICE, LOSS_THRESHOLD);
        assert!(
            split > Fixed::ZERO,
            "the receptive learner: perceive outstrips produce"
        );

        // The swap gives the mirror: an intact voice (production full) and an ear tuned to the
        // earlier production level (perception equal to that production), so the split is exactly
        // negated. Two halves swapped, one kernel.
        let prod = halves.production;
        let intact = body();
        let tuned_ear = Sensorium::with([(VOICE, prod)]);
        let mirror = acquisition_split(&intact, F_VITAL_CORE, &tuned_ear, VOICE, LOSS_THRESHOLD);
        assert_eq!(
            mirror.to_bits(),
            -split.to_bits(),
            "the swap gives the mirror split"
        );

        // Equal halves split to zero: a full voice and a full ear.
        let full = body();
        let matched = Sensorium::with([(VOICE, Fixed::ONE)]);
        let zero = acquisition_split(&full, F_VITAL_CORE, &matched, VOICE, LOSS_THRESHOLD);
        assert_eq!(zero, Fixed::ZERO, "equal halves split zero");

        // Two races with identical body and sensorium data split identically: no race id enters.
        let a = acquisition_split(&body(), F_VITAL_CORE, &matched, VOICE, LOSS_THRESHOLD);
        let b = acquisition_split(&body(), F_VITAL_CORE, &matched, VOICE, LOSS_THRESHOLD);
        assert_eq!(a, b, "identical data splits to identical bits");
    }
}
