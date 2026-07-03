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

//! The GPU being-kernel gate: `gpu_body_thermal` must reproduce the runner's `phase_body_exchange`
//! (discrete Newton cooling of each being toward its cell temperature) bit-for-bit. The CPU reference
//! is the exact same update written against `civsim_core::Fixed`. Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_body_thermal};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// The exact CPU reference: `next = bt + exchange * (env - bt)` per being.
fn cpu_body_thermal(field: &[i64], cell: &[u32], body_temp: &[i64], exchange: Fixed) -> Vec<i64> {
    body_temp
        .iter()
        .zip(cell)
        .map(|(&bt, &c)| {
            let bt = Fixed::from_bits(bt);
            let env = Fixed::from_bits(field[c as usize]);
            (bt + exchange.mul(env - bt)).to_bits()
        })
        .collect()
}

#[test]
fn body_thermal_is_bit_identical_to_phase_body_exchange() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping being gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    // A field with a spread of positive and negative temperatures, so `env - bt` takes both signs and
    // exercises the pinned multiply's sign handling.
    let (w, h) = (20usize, 12usize);
    let field: Vec<i64> = (0..w * h)
        .map(|i| Fixed::from_ratio(((i * 7) % 40) as i64 - 20, 3).to_bits())
        .collect();
    let n = 500usize;
    let cell: Vec<u32> = (0..n).map(|i| ((i * 13) % (w * h)) as u32).collect();
    let body_temp: Vec<i64> = (0..n)
        .map(|i| Fixed::from_ratio(((i * 5) % 30) as i64 - 10, 4).to_bits())
        .collect();
    let exchange = Fixed::from_ratio(1, 8);

    let got = gpu_body_thermal(&client, &field, &cell, &body_temp, exchange.to_bits());
    let want = cpu_body_thermal(&field, &cell, &body_temp, exchange);
    assert_eq!(got.len(), want.len());
    let mism = got.iter().zip(&want).filter(|(a, b)| a != b).count();
    assert_eq!(
        mism, 0,
        "GPU body-thermal must equal phase_body_exchange over all {n} beings"
    );
}
