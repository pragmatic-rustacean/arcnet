use serde::Deserialize;
use uint::construct_uint;

construct_uint! {
  #[derive(serde::Serialize, Deserialize)]
  pub(crate) struct U256(4);
}

pub mod crypto;
pub mod sha256;
pub mod types;
pub mod util;
