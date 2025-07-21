#![no_main]

mod common;
use common::*;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo::console;
use rand::rngs::OsRng;
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::{pkcs1v15::Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

fn error_report(message: &str) {
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
    global
        .post_message(
            &serde_wasm_bindgen::to_value(&WorkerMessage::Error {
                message: message.to_string(),
            })
            .unwrap(),
        )
        .unwrap();
}

#[wasm_bindgen(start)]
pub fn worker_main() {
    console::log!("🔧 Worker started");
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

    let global_clone = global.clone();
    let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: MessageEvent| {
        let global_inner = global_clone.clone();

        // エラーハンドリングを改善
        match serde_wasm_bindgen::from_value::<MainMessage>(event.data()) {
            Ok(message) => {
                console::log!("🔧 Worker message received");
                spawn_local(async move {
                    match message {
                        MainMessage::GenerateKeyPair => {
                            match generate_key_pair_with_progress().await {
                                Ok((public_key, private_key)) => {
                                    match serde_wasm_bindgen::to_value(&WorkerMessage::Generated {
                                        public_key,
                                        private_key,
                                    }) {
                                        Ok(message) => {
                                            console::log!("🔧 Sending message to main thread");
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting message: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing generated message: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error generating key pair: {:?}",
                                        e.to_string()
                                    ));
                                }
                            }
                        }
                        MainMessage::Encrypt { public_key, data } => {
                            console::log!("🔧 Encrypting message");
                            match encrypt_message(&public_key, &data) {
                                Ok(encrypted) => {
                                    match serde_wasm_bindgen::to_value(&WorkerMessage::Encrypted {
                                        encrypted_data: encrypted,
                                    }) {
                                        Ok(message) => {
                                            console::log!(
                                                "🔧 Sending encrypted message to main thread"
                                            );
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting encrypted message: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing encrypted message: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error encrypting message: {:?}",
                                        e.to_string()
                                    ));
                                }
                            }
                        }
                        MainMessage::Decrypt { private_key, data } => {
                            console::log!("🔧 Decrypting message");
                            match decrypt_message(&private_key, &data) {
                                Ok(Some(decrypted)) => {
                                    match serde_wasm_bindgen::to_value(&WorkerMessage::Decrypted {
                                        decrypted_data: decrypted,
                                    }) {
                                        Ok(message) => {
                                            console::log!(
                                                "🔧 Sending decrypted message to main thread"
                                            );
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting decrypted message: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing decrypted message: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Ok(None) => {
                                    error_report("❌ Error decrypting message");
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error decrypting message: {:?}",
                                        e.to_string()
                                    ));
                                }
                            }
                        }
                    }
                });
            }
            Err(e) => {
                error_report(&format!("❌ Error deserializing worker message: {:?}", e));
            }
        }
    }) as Box<dyn FnMut(_)>);

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // WorkerMessage::Readyの送信もエラーハンドリングを追加
    match serde_wasm_bindgen::to_value(&WorkerMessage::Ready) {
        Ok(ready_message) => {
            if let Err(e) = global.post_message(&ready_message) {
                error_report(&format!("❌ Error posting ready message: {:?}", e));
            }
        }
        Err(e) => {
            error_report(&format!("❌ Error serializing ready message: {:?}", e));
        }
    }
}

async fn generate_key_pair_with_progress() -> Result<(String, String), Box<dyn std::error::Error>> {
    console::log!("🔧 RSA key generation process started");

    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;
    let public_key = RsaPublicKey::from(&private_key);

    let private_pem = private_key.to_pkcs1_der()?;
    let public_pem = public_key.to_pkcs1_der()?;

    let private_key_str = BASE64.encode(private_pem.as_bytes());
    let public_key_str = BASE64.encode(public_pem.as_bytes());

    console::log!("✅ RSA key pair successfully generated");

    Ok((public_key_str, private_key_str))
}

fn encrypt_message(public_key: &str, message: &str) -> Result<String, Box<dyn std::error::Error>> {
    console::log!("🔑 Decoding public key from Base64...");
    let public_key_bytes = BASE64.decode(public_key)?;

    console::log!("🔍 Parsing RSA public key...");
    let public_key = RsaPublicKey::from_pkcs1_der(&public_key_bytes)?;

    console::log!("🎲 Generating random number...");
    let mut rng = OsRng;

    console::log!("🔐 Encrypting message...");
    let padding = Pkcs1v15Encrypt;
    let encrypted = public_key.encrypt(&mut rng, padding, message.as_bytes())?;

    console::log!("📦 Encoding to Base64...");
    let result = BASE64.encode(&encrypted);

    console::log!(&format!("✅ Encryption completed: {} bytes", result.len()));
    Ok(result)
}

fn decrypt_message(
    private_key: &str,
    encrypted_message: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    console::log!("🔑 Decoding private key from Base64...");
    let private_key_bytes = BASE64.decode(private_key)?;

    console::log!("🔍 Parsing RSA private key...");
    let private_key = RsaPrivateKey::from_pkcs1_der(&private_key_bytes)?;

    console::log!("📦 Decoding encrypted message from Base64...");
    let encrypted_bytes = BASE64.decode(encrypted_message)?;

    console::log!("🔓 Decrypting message...");
    let decrypted = match private_key.decrypt(Pkcs1v15Encrypt, &encrypted_bytes) {
        Ok(decrypted) => decrypted,
        Err(e) => {
            console::error!(&format!("❌ Error decrypting message: {:?}", e));
            return Ok(None);
        }
    };

    console::log!("📝 Converting decrypted bytes to string...");
    let result = String::from_utf8(decrypted)?;

    console::log!(&format!("✅ Decryption completed: {} chars", result.len()));
    Ok(Some(result))
}
