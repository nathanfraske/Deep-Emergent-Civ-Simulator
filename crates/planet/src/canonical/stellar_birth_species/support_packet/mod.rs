//! Production-unreachable structural packet for future species support.
//!
//! The pair checks canonical bytes, complete support disposition, exact
//! normalization, binding consistency, and bounded execution. It does not
//! establish that any referenced digest is physical evidence, and it returns
//! no proof token or registry authority.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use model::{PacketRefusal, SpeciesSupportPacket};

fn inspect_packet(packet: &SpeciesSupportPacket) -> Result<Vec<u8>, PacketRefusal> {
    let produced = producer::validate_and_encode(packet)?;
    let watched = watchdog::validate_and_encode(packet)?;
    if produced != watched {
        return Err(PacketRefusal::CheckerDisagreement);
    }
    Ok(produced)
}
