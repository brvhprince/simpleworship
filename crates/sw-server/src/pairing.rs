//! Connecting a new device, safely:
//!
//! 1. The operator clicks "Connect a Phone": a 6-digit code is made, valid
//!    for five minutes ([`Pairing::new_code`]).
//! 2. The phone sends the code and its name. If the code is right, the
//!    operator is asked whether to allow it, and as which role.
//! 3. If allowed, the phone gets a long random token and keeps it. The
//!    computer keeps only a hash of it (a [`Device`]), so the saved list of
//!    devices cannot be used to connect.
//!
//! Every later connection shows the token; [`find_device`] checks it.

use std::time::{Duration, Instant};

use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sw_core::{Id, time};
use ts_rs::TS;

use crate::roles::Role;

/// How long a pairing code works.
pub const CODE_LIFETIME: Duration = Duration::from_secs(5 * 60);

/// How many wrong codes are accepted before the current code is thrown away.
const MAX_WRONG_CODES: u32 = 10;

/// A device allowed to connect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "remote.ts")]
pub struct Device {
    /// Identifies the device.
    pub id: Id,
    /// The name the phone gave, e.g. "Pastor's iPhone".
    pub name: String,
    /// What it may do.
    pub role: Role,
    /// SHA-256 of its token, as hex. The token itself is only on the phone.
    pub token_hash: String,
    /// When it was allowed.
    #[ts(type = "number")]
    pub added_at: i64,
    /// When it last connected.
    #[ts(type = "number")]
    pub last_seen: i64,
}

/// The current pairing code, if the operator has asked for one.
#[derive(Debug, Default)]
pub struct Pairing {
    code: Option<(String, Instant)>,
    wrong: u32,
}

impl Pairing {
    /// Makes a new 6-digit code, replacing any earlier one.
    pub fn new_code(&mut self) -> String {
        let code = format!("{:06}", rand::rng().random_range(0..1_000_000));
        self.code = Some((code.clone(), Instant::now()));
        self.wrong = 0;
        code
    }

    /// Checks a code a phone sent. A right code can be used once; too many
    /// wrong ones cancel it, so it cannot be guessed.
    pub fn check(&mut self, attempt: &str) -> bool {
        let Some((code, made)) = &self.code else {
            return false;
        };
        if made.elapsed() > CODE_LIFETIME {
            self.code = None;
            return false;
        }
        if attempt.trim() == code {
            self.code = None;
            return true;
        }
        self.wrong += 1;
        if self.wrong >= MAX_WRONG_CODES {
            self.code = None;
        }
        false
    }

    /// Forgets the code (the operator closed the "Connect a Phone" window).
    pub fn cancel(&mut self) {
        self.code = None;
    }
}

/// A new random token for a device.
pub fn new_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    hex(&bytes)
}

/// The hash of a token, as stored in a [`Device`].
pub fn hash_token(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// The device a token belongs to, if any.
pub fn find_device<'a>(devices: &'a [Device], token: &str) -> Option<&'a Device> {
    let hash = hash_token(token);
    devices.iter().find(|device| device.token_hash == hash)
}

/// A device record for a newly allowed phone, and the token to give it.
pub fn new_device(name: &str, role: Role) -> (Device, String) {
    let token = new_token();
    let now = time::now();
    let name = name.trim();
    let device = Device {
        id: Id::new(),
        name: if name.is_empty() {
            "Phone".into()
        } else {
            name.chars().take(60).collect()
        },
        role,
        token_hash: hash_token(&token),
        added_at: now,
        last_seen: now,
    };
    (device, token)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_works_once() {
        let mut pairing = Pairing::default();
        let code = pairing.new_code();
        assert_eq!(code.len(), 6);
        assert!(!pairing.check("000000x"));
        assert!(pairing.check(&code));
        assert!(!pairing.check(&code), "used up");
    }

    #[test]
    fn too_many_wrong_codes_cancel_it() {
        let mut pairing = Pairing::default();
        let code = pairing.new_code();
        for _ in 0..MAX_WRONG_CODES {
            pairing.check("not it");
        }
        assert!(!pairing.check(&code));
    }

    #[test]
    fn devices_are_found_by_token_but_only_its_hash_is_kept() {
        let (device, token) = new_device("Pastor's iPhone", Role::Presenter);
        assert_ne!(device.token_hash, token);
        let devices = vec![device.clone()];
        assert_eq!(find_device(&devices, &token), Some(&device));
        assert_eq!(find_device(&devices, "guess"), None);
    }
}
