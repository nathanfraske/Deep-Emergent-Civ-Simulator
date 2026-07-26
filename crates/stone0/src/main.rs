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

//! `stone0-gate`: the Stone 0 provenance gate binary. CI runs it with `--ci`, its self-proof with
//! `--self-test`, and a developer runs it with no flag for the full local check. The canonical planet
//! build anchor and parked compatibility runner also call the library from build scripts.

use civsim_stone0::{run, Mode};
use std::ffi::OsString;

const USAGE: &str = "usage: stone0-gate [--ci | --self-test]";

fn parse_mode(arguments: &[OsString]) -> Result<Mode, ()> {
    match arguments {
        [] => Ok(Mode::Local),
        [flag] if flag == "--ci" => Ok(Mode::Ci),
        [flag] if flag == "--self-test" => Ok(Mode::SelfTest),
        _ => Err(()),
    }
}

fn main() {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mode = match parse_mode(&arguments) {
        Ok(mode) => mode,
        Err(()) => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    std::process::exit(run(mode));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn exact_cli_modes_are_accepted() {
        assert_eq!(parse_mode(&[]), Ok(Mode::Local));
        assert_eq!(parse_mode(&arguments(&["--ci"])), Ok(Mode::Ci));
        assert_eq!(parse_mode(&arguments(&["--self-test"])), Ok(Mode::SelfTest));
    }

    #[test]
    fn conflicting_duplicate_unknown_and_positional_arguments_are_rejected() {
        for invalid in [
            arguments(&["--ci", "--self-test"]),
            arguments(&["--ci", "--ci"]),
            arguments(&["--self-test", "--self-test"]),
            arguments(&["--unknown"]),
            arguments(&["positional"]),
            arguments(&["--ci", "positional"]),
        ] {
            assert_eq!(parse_mode(&invalid), Err(()));
        }
    }
}
