use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs};
fn verify(
    key: &str,
    signature: &str,
    data: &[u8],
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let public = String::from_utf8(STANDARD.decode(key.trim())?)?;
    let signed = String::from_utf8(STANDARD.decode(signature.trim())?)?;
    let public = PublicKey::decode(&public)?;
    let signature = Signature::decode(&signed)?;
    public.verify(data, &signature, false)?;
    if !signature
        .trusted_comment()
        .split_whitespace()
        .any(|item| item == format!("version:{version}"))
    {
        return Err("Signature does not bind the announced version".into());
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = env::args().collect();
    if a.len() != 5 {
        return Err("Expected public-key file, signature file, artifact file and version".into());
    }
    verify(
        &fs::read_to_string(&a[1])?,
        &fs::read_to_string(&a[2])?,
        &fs::read(&a[3])?,
        &a[4],
    )?;
    println!("Artifact signature and version verified");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_data_rejects_tampering_wrong_key_and_version() {
        let key = include_str!("../../tests/updater-fixtures/public.txt");
        let signature = include_str!("../../tests/updater-fixtures/payload.txt.sig");
        let data = include_bytes!("../../tests/updater-fixtures/payload.txt");
        assert!(verify(key, signature, data, "0.5.1").is_ok());
        assert!(verify(key, signature, b"tampered", "0.5.1").is_err());
        assert!(verify(key, signature, data, "99.0.0").is_err());
        assert!(
            verify(
                include_str!("../../tests/updater-fixtures/wrong-public.txt"),
                signature,
                data,
                "0.5.1"
            )
            .is_err()
        );
        assert!(verify(key, "", data, "0.5.1").is_err());
    }
}
