#![allow(unused)]
use ecdsa::{
    SigningKey, VerifyingKey,
    der::Signature as ECDSASignature,
    signature::{SignerMut, Verifier},
};
use k256::Secp256k1;
use rand::rngs;
use serde::{Deserialize, Serialize};

use crate::sha256::Hash;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Signature(ECDSASignature<Secp256k1>);
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub(crate) struct PublicKey(VerifyingKey<Secp256k1>);
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PrivateKey(#[serde(with = "signkey_serde")] pub SigningKey<Secp256k1>);

mod signkey_serde {
    use ecdsa::SigningKey;
    use k256::Secp256k1;
    use serde::Deserialize;

    pub(crate) fn serialize<S>(
        key: &SigningKey<Secp256k1>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<SigningKey<Secp256k1>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::<u8>::deserialize(deserializer)?;
        Ok(SigningKey::from_slice(&bytes)
            .expect("Failed to deserialize the data, @crypt.rs/line33"))
    }
}

impl PrivateKey {
    pub(crate) fn new() -> Self {
      /// use OsRng since thread_rng | rng doesn't implement CryptoRngCore. 
        let mut rng = rngs::OsRng;
        Self(SigningKey::random(&mut rng))
    }
    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().clone())
    }
}

impl Signature {
    pub(crate) fn sign_output(output_hash: &Hash, private_key: &PrivateKey) -> Self {
        let mut signing_key = private_key.0.clone();
        let signature = signing_key.sign(&output_hash.as_bytes());
        Self(signature)
    }

    pub(crate) fn verify(&self, output_hash: &Hash, public_key: &PublicKey) -> bool {
        public_key
            .0
            .verify(&output_hash.as_bytes(), &self.0)
            .is_ok()
    }
}
