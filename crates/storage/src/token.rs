//! Bearer credentials for the hub: the link token handed out by `/link/new` and
//! the per-subscriber secret minted on bind.
//!
//! Both are 256 bits from a CSPRNG, handed to the client base64url-encoded, and
//! persisted only as a SHA-256 digest. A plaintext credential therefore exists
//! in exactly one place — the client that received it — so a dump of the hub
//! database yields nothing that can authenticate.
//!
//! SHA-256 rather than a password KDF (argon2/bcrypt) is deliberate: the input
//! is 256 bits of uniform randomness, not a human-chosen password, so there is
//! no dictionary to grind and a deliberately slow hash would only add latency to
//! every authenticated request.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

const TOKEN_BYTES: usize = 32;

/// A freshly minted credential. `plaintext` goes to the client exactly once;
/// `hash` is the only half that touches the database.
pub(crate) struct Token {
    pub(crate) plaintext: String,
    pub(crate) hash: String,
}

/// Mint a new credential from the thread-local CSPRNG.
pub(crate) fn mint() -> Token {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    let plaintext = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash(&plaintext);
    Token { plaintext, hash }
}

/// The at-rest form of `plaintext`, used both when storing a fresh credential
/// and when looking one up on an incoming request.
pub(crate) fn hash(plaintext: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(plaintext.as_bytes()))
}
