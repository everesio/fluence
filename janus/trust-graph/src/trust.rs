/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::key_pair::{KeyPair, Signature};
use libp2p_core::identity::ed25519::PublicKey;
use std::convert::TryInto;
use std::time::Duration;

pub const SIGNATURE_LEN: usize = 64;
pub const PUBLIC_KEY_LEN: usize = 32;
pub const EXPIRATION_LEN: usize = 8;
pub const ISSUED_LEN: usize = 8;
pub const TRUST_LEN: usize = SIGNATURE_LEN + PUBLIC_KEY_LEN + EXPIRATION_LEN + ISSUED_LEN;

/// One element in chain of trust in a certificate.
/// TODO delete pk from Trust (it is already in a trust node)
#[derive(Clone, Debug, PartialEq)]
pub struct Trust {
    pub pk: PublicKey,
    /// Expiration date of a trust.
    pub expires_at: Duration,
    /// Signature of a previous trust in a chain.
    /// Signature is self-signed if it is a root trust.
    pub signature: Signature,

    pub issued_at: Duration,
}

impl Trust {
    #[allow(dead_code)]
    pub fn new(
        pk: PublicKey,
        expires_at: Duration,
        issued_at: Duration,
        signature: Signature,
    ) -> Self {
        Self {
            pk,
            expires_at,
            issued_at,
            signature,
        }
    }

    pub fn create(
        issued_by: &KeyPair,
        pk: PublicKey,
        expires_at: Duration,
        issued_at: Duration,
    ) -> Self {
        let msg = Self::trust_to_bytes(pk.clone(), expires_at, issued_at);

        let signature = issued_by.sign(&msg);

        Self {
            pk,
            expires_at,
            signature,
            issued_at,
        }
    }

    /// Verifies that authorization is cryptographically correct.
    pub fn verify(trust: &Trust, issued_by: &PublicKey, time: Duration) -> Result<(), String> {
        if trust.expires_at < time {
            return Err("Trust in chain is expired.".to_string());
        }

        let msg = Self::trust_to_bytes(trust.pk.clone(), trust.expires_at, trust.issued_at);

        let verify_result = KeyPair::verify(issued_by, &msg, trust.signature.as_slice());
        if !verify_result {
            return Err("Trust in chain is forged.".to_string());
        }

        Ok(())
    }

    fn trust_to_bytes(pk: PublicKey, expires_at: Duration, issued_at: Duration) -> [u8; 48] {
        let pk_encoded = pk.encode();
        let expires_at_encoded: [u8; 8] = (expires_at.as_millis() as u64).to_le_bytes();
        let issued_at_encoded: [u8; 8] = (issued_at.as_millis() as u64).to_le_bytes();
        let mut msg = [0; 48];

        msg[..32].clone_from_slice(&pk_encoded[..32]);
        msg[33..40].clone_from_slice(&expires_at_encoded[0..7]);
        msg[41..48].clone_from_slice(&issued_at_encoded[0..7]);

        msg
    }

    /// Encode the trust into a byte array
    #[allow(dead_code)]
    pub fn encode(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(TRUST_LEN);
        vec.extend_from_slice(&self.pk.encode());
        vec.extend_from_slice(&self.signature.as_slice());
        vec.extend_from_slice(&(self.expires_at.as_millis() as u64).to_le_bytes());
        vec.extend_from_slice(&(self.issued_at.as_millis() as u64).to_le_bytes());

        vec
    }

    /// Decode a trust from a byte array as produced by `encode`.
    #[allow(dead_code)]
    pub fn decode(arr: &[u8]) -> Result<Self, String> {
        if arr.len() != TRUST_LEN {
            return Err(
                "Trust length should be 104: public key(32) + signature(64) + expiration date(8)"
                    .to_string(),
            );
        }

        let pk = PublicKey::decode(&arr[0..PUBLIC_KEY_LEN]).map_err(|err| err.to_string())?;
        let signature = &arr[PUBLIC_KEY_LEN..PUBLIC_KEY_LEN + SIGNATURE_LEN];

        let expiration_bytes =
            &arr[PUBLIC_KEY_LEN + SIGNATURE_LEN..PUBLIC_KEY_LEN + SIGNATURE_LEN + EXPIRATION_LEN];
        let expiration_date = u64::from_le_bytes(expiration_bytes.try_into().unwrap());
        let expiration_date = Duration::from_millis(expiration_date);

        let issued_bytes = &arr[PUBLIC_KEY_LEN + SIGNATURE_LEN + EXPIRATION_LEN..TRUST_LEN];
        let issued_date = u64::from_le_bytes(issued_bytes.try_into().unwrap());
        let issued_date = Duration::from_millis(issued_date);

        Ok(Self {
            pk,
            signature: signature.to_vec(),
            expires_at: expiration_date,
            issued_at: issued_date,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_gen_revoke_and_validate() {
        let truster = KeyPair::generate();
        let trusted = KeyPair::generate();

        let current = Duration::new(100, 0);
        let duration = Duration::new(1000, 0);
        let issued_at = Duration::new(10, 0);

        let trust = Trust::create(&truster, trusted.show_public_key(), duration, issued_at);

        assert_eq!(
            Trust::verify(&trust, &truster.show_public_key(), current).is_ok(),
            true
        );
    }

    #[test]
    fn test_validate_corrupted_revoke() {
        let truster = KeyPair::generate();
        let trusted = KeyPair::generate();

        let current = Duration::new(1000, 0);
        let issued_at = Duration::new(10, 0);

        let trust = Trust::create(&truster, trusted.show_public_key(), current, issued_at);

        let corrupted_duration = Duration::new(1234, 0);
        let corrupted_trust = Trust::new(
            trust.pk,
            trust.expires_at,
            corrupted_duration,
            trust.signature,
        );

        assert!(Trust::verify(&corrupted_trust, &truster.show_public_key(), current).is_err());
    }

    #[test]
    fn test_encode_decode() {
        let truster = KeyPair::generate();
        let trusted = KeyPair::generate();

        let current = Duration::new(1000, 0);
        let issued_at = Duration::new(10, 0);

        let trust = Trust::create(&truster, trusted.show_public_key(), current, issued_at);

        let encoded = trust.encode();
        let decoded = Trust::decode(encoded.as_slice()).unwrap();

        assert_eq!(trust, decoded);
    }
}
