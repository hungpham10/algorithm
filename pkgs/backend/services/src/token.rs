use std::io::{Error, ErrorKind};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::{RngCore, thread_rng};

pub async fn run(master_key: &String, action: &str, payload: &String) -> std::io::Result<()> {
    if master_key.len() != 32 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Master key must have length equal to 32, now it is {}",
                master_key.len(),
            ),
        ));
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);

    match action {
        "encrypt" => {
            let mut nonce_bytes = [0u8; 12];
            thread_rng().fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher.encrypt(nonce, payload.as_bytes()).map_err(|error| {
                Error::new(ErrorKind::BrokenPipe, format!("Encrypt failed: {error}"))
            })?;

            let mut final_blob = nonce_bytes.to_vec();
            final_blob.extend_from_slice(&ciphertext);

            println!("{}", hex::encode(final_blob));
            Ok(())
        }
        "decrypt" => {
            let encrypted_bytes = hex::decode(payload).map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Convert to bytes failed: {error}"),
                )
            })?;
            if encrypted_bytes.len() < 12 {
                return Err(Error::new(ErrorKind::BrokenPipe, "Data too short"));
            }

            let (nonce_part, ciphertext_part) = encrypted_bytes.split_at(12);
            let nonce = Nonce::from_slice(nonce_part);

            let decrypted_bytes = cipher.decrypt(nonce, ciphertext_part).map_err(|error| {
                Error::new(ErrorKind::BrokenPipe, format!("Decrypt failed: {error}"))
            })?;

            let plain_str = String::from_utf8(decrypted_bytes).map_err(|error| {
                Error::new(ErrorKind::BrokenPipe, format!("Data is not UTF8: {error}"))
            })?;
            println!("{}", plain_str);
            Ok(())
        }
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            "Unknown action, only 'encrypt' or 'decrypt'",
        )),
    }
}
