use crate::{sha256::Hash, util::Saveable};
use ecdsa::{
    SigningKey, VerifyingKey,
    der::Signature as ECDSASignature,
    signature::{SignerMut, Verifier},
};
use k256::{Secp256k1, pkcs8::EncodePublicKey};
use rand::rngs;
use serde::{Deserialize, Serialize};
use std::io::{Error as IOError, ErrorKind};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signature(ECDSASignature<Secp256k1>);
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey<Secp256k1>);
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivateKey(#[serde(with = "signkey_serde")] pub SigningKey<Secp256k1>);

impl Saveable for PrivateKey {
    fn load<R: std::io::Read>(reader: R) -> std::io::Result<Self> {
        ciborium::from_reader(reader)
            .map_err(|_| IOError::new(ErrorKind::InvalidData, "Failed to deserialize private key"))
    }

    fn save<W: std::io::Write>(&self, writer: W) -> std::io::Result<()> {
        ciborium::into_writer(&self, writer)
            .map_err(|_| IOError::new(ErrorKind::InvalidData, "Failed to serialize private key"))?;

        Ok(())
    }
}

impl Saveable for PublicKey {
    fn load<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        // Read PEM-encoded public key into string.
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        let public_key = buf.parse().map_err(|_| {
            IOError::new(ErrorKind::InvalidData, "Failed to deserialize public key")
        })?;

        Ok(PublicKey(public_key))
    }

    fn save<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        let s = self
            .0
            .to_public_key_pem(Default::default())
            .map_err(|_| IOError::new(ErrorKind::InvalidData, "Failed to serialize public key"))?;

        let _ = writer.write_all(s.as_bytes())?;
        Ok(())
    }
}

mod signkey_serde {
    use ecdsa::SigningKey;
    use k256::Secp256k1;
    use serde::Deserialize;

    pub fn serialize<S>(key: &SigningKey<Secp256k1>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SigningKey<Secp256k1>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::<u8>::deserialize(deserializer)?;
        Ok(SigningKey::from_slice(&bytes)
            .expect("Failed to deserialize the data, @crypt.rs/line33"))
    }
}

impl PrivateKey {
    pub fn new() -> Self {
        // use OsRng since thread_rng | rng doesn't implement CryptoRngCore.
        let mut rng = rngs::OsRng;
        Self(SigningKey::random(&mut rng))
    }
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().clone())
    }
}

impl Signature {
    pub fn sign_output(output_hash: &Hash, private_key: &PrivateKey) -> Self {
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
