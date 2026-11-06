// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::error::{Error, Result};
use minisign_verify::{PublicKey, Signature};

const PUBLIC_KEY: &str = "RWQIISBNpKRKRP5ILebS96X44Nh05784HVJdk71MBLHvoezrKP1rgv6t";

pub fn verify(exe: &[u8], sig: &[u8], tag: &str) -> Result<()> {
    verify_with(PUBLIC_KEY, exe, sig, tag)
}

fn verify_with(key: &str, exe: &[u8], sig: &[u8], tag: &str) -> Result<()> {
    let refused = || Error::Other("this update isn't signed by rbxyoinker, so it wasn't installed".into());
    let key = PublicKey::from_base64(key).map_err(|_| refused())?;
    let text = std::str::from_utf8(sig).map_err(|_| refused())?;
    let sig = Signature::decode(text).map_err(|_| refused())?;
    key.verify(exe, &sig, false).map_err(|_| refused())?;
    if sig.trusted_comment() != format!("rbxyoinker {tag}") {
        return Err(refused());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use minisign::KeyPair;
    use std::io::Cursor;

    const EXE: &[u8] = b"MZ pretend this is an exe";

    fn keys() -> (String, KeyPair) {
        let pair = KeyPair::generate_unencrypted_keypair().unwrap();
        (pair.pk.to_base64(), pair)
    }

    fn sign(pair: &KeyPair, exe: &[u8], comment: &str) -> Vec<u8> {
        minisign::sign(Some(&pair.pk), &pair.sk, Cursor::new(exe), Some(comment), None)
            .unwrap()
            .to_string()
            .into_bytes()
    }

    #[test]
    fn real_key_is_set() {
        assert!(PublicKey::from_base64(PUBLIC_KEY).is_ok());
    }

    #[test]
    fn good_signature_passes() {
        let (pk, pair) = keys();
        assert!(verify_with(&pk, EXE, &sign(&pair, EXE, "rbxyoinker v0.3.2"), "v0.3.2").is_ok());
    }

    #[test]
    fn changed_exe_is_refused() {
        let (pk, pair) = keys();
        let sig = sign(&pair, EXE, "rbxyoinker v0.3.2");
        assert!(verify_with(&pk, b"MZ something else", &sig, "v0.3.2").is_err());
    }

    #[test]
    fn missing_signature_is_refused() {
        let (pk, _) = keys();
        assert!(verify_with(&pk, EXE, b"", "v0.3.2").is_err());
    }

    #[test]
    fn wrong_key_is_refused() {
        let (_, pair) = keys();
        let (other, _) = keys();
        assert!(verify_with(&other, EXE, &sign(&pair, EXE, "rbxyoinker v0.3.2"), "v0.3.2").is_err());
    }

    #[test]
    fn old_signed_exe_under_new_tag_is_refused() {
        let (pk, pair) = keys();
        assert!(verify_with(&pk, EXE, &sign(&pair, EXE, "rbxyoinker v0.3.2"), "v0.3.3").is_err());
    }

    #[test]
    fn edited_trusted_comment_is_refused() {
        let (pk, pair) = keys();
        let sig = String::from_utf8(sign(&pair, EXE, "rbxyoinker v0.3.2")).unwrap();
        let forged = sig.replace("rbxyoinker v0.3.2", "rbxyoinker v0.3.3");
        assert!(verify_with(&pk, EXE, forged.as_bytes(), "v0.3.3").is_err());
    }

    #[test]
    fn placeholder_key_fails_closed() {
        let (_, pair) = keys();
        let sig = sign(&pair, EXE, "rbxyoinker v0.3.2");
        assert!(verify_with("SET_BEFORE_RELEASE", EXE, &sig, "v0.3.2").is_err());
    }
}
