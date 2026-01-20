use serde::Deserialize;
use uint::construct_uint;

construct_uint! {
  ///### construct an unsigned 256-bit interger consisting of 4 x 64-bit words.
  #[derive(serde::Serialize, Deserialize)]
  pub struct U256(4);
}

pub mod crypto;
pub mod error;
pub mod network;
pub mod sha256;
pub mod types;
pub mod util;

///### initial reward in bitcoin - multply by 10^8 to get satoshis'
pub const INITIAL_BLOCK_REWARD: u64 = 50;
///### halving intervals in blocks.
pub(crate) const HALVING_INTERVAL: u64 = 240;
/// ideal block time in secs.
pub(crate) const IDEAL_BLOCK_TIME: u64 = 10;
///### minimum target.
/// Our minimum target only requires the first four hex to be zero.
pub const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_FFFF_FFFF_FFFF,
]);
///### Difficulty update interval in blocks.
/// Difficulty is how unlikely it should be to encounter the correct hash while mining.,
/// unlike real Bitcoin which has an update interval of 2016, i'm using 50 for development purposes, so that I don't wait a long time to see if my code works.
pub(crate) const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;
/// maximum mempool transactions in secs
pub(crate) const MAX_MEMPOOL_TRANSACTION_AGE: u64 = 600;
/// Maximum amount of transactions allowed for a block
pub const BLOCK_TRANSACTION_CAP: usize = 20;
