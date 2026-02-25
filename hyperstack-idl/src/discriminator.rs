//! Discriminator utilities

use sha2::{Digest, Sha256};

pub fn anchor_discriminator(preimage: &str) -> Vec<u8> {
    let hash = Sha256::digest(preimage.as_bytes());
    hash[..8].to_vec()
}
