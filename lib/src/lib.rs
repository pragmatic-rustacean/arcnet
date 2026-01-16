#![allow(unused)]
use serde::Deserialize;
use uint::construct_uint;

construct_uint! {
  /// construct an unsigned 256-bit interger consisting of 4 x 64-bit words.
  #[derive(serde::Serialize, Deserialize)]
  pub(crate) struct U256(4);
}

pub mod crypto;
pub mod error;
pub mod sha256;
pub mod types;
pub mod util;

/// initial reward in bitcoin - multply by 10^8 to get satoshis;
pub(crate) const INITIAL_BLOCK_REWARD: u64 = 50;
/// halving intervals in blocks.
pub(crate) const HALVING_INTERVAL: u64 = 240;
/// ideal block time in secs.
pub(crate) const IDEAL_BLOCK_TIME: u64 = 10;
/// minimum target
pub(crate) const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
]);
/// difficulty update interval in blocks.
pub(crate) const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;
