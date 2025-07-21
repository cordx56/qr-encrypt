#![no_main]

mod common;
use common::*;

use age::{x25519, Decryptor, Encryptor};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo::console;
use secrecy::ExposeSecret;
use std::io::{Read, Write};
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
                        MainMessage::ExportPrivateKey {
                            recipient_public_key,
                            private_key,
                        } => {
                            console::log!("🔧 Exporting private key");
                            match encrypt_message(&recipient_public_key, &private_key) {
                                Ok(encrypted_private_key) => {
                                    match serde_wasm_bindgen::to_value(
                                        &WorkerMessage::PrivateKeyExported {
                                            encrypted_private_key,
                                        },
                                    ) {
                                        Ok(message) => {
                                            console::log!(
                                                "🔧 Sending exported private key to main thread"
                                            );
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting exported private key: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing exported private key: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error exporting private key: {:?}",
                                        e.to_string()
                                    ));
                                }
                            }
                        }
                        MainMessage::GeneratePublicKeyFromPrivate { private_key } => {
                            console::log!("🔧 Generating public key from private key");
                            match generate_public_key_from_private(&private_key) {
                                Ok(public_key) => {
                                    match serde_wasm_bindgen::to_value(
                                        &WorkerMessage::PublicKeyGenerated { public_key },
                                    ) {
                                        Ok(message) => {
                                            console::log!(
                                                "🔧 Sending generated public key to main thread"
                                            );
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting generated public key: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing generated public key: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error generating public key: {:?}",
                                        e.to_string()
                                    ));
                                }
                            }
                        }
                        MainMessage::ProcessQrData { data } => {
                            console::log!("🔧 Processing QR data");
                            match process_qr_data(&data) {
                                Ok((event_type, event_data)) => {
                                    match serde_wasm_bindgen::to_value(
                                        &WorkerMessage::QrDataProcessed {
                                            event_type,
                                            event_data,
                                        },
                                    ) {
                                        Ok(message) => {
                                            console::log!(
                                                "🔧 Sending processed QR data to main thread"
                                            );
                                            if let Err(e) = global_inner.post_message(&message) {
                                                error_report(&format!(
                                                    "❌ Error posting processed QR data: {:?}",
                                                    e
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            error_report(&format!(
                                                "❌ Error serializing processed QR data: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_report(&format!(
                                        "❌ Error processing QR data: {:?}",
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
    console::log!("🔧 X25519 key generation process started");

    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();

    // age鍵は文字列として直接出力できます
    let private_key_str = identity.to_string().expose_secret().to_string();
    let public_key_str = recipient.to_string();

    console::log!("✅ X25519 key pair successfully generated");

    Ok((public_key_str, private_key_str))
}

fn encrypt_message(public_key: &str, message: &str) -> Result<String, Box<dyn std::error::Error>> {
    console::log!("🔑 Parsing X25519 public key...");
    let recipient: x25519::Recipient = public_key.parse()?;

    console::log!("🔐 Encrypting message...");
    let encryptor = Encryptor::with_recipients(
        vec![Box::new(recipient) as Box<dyn age::Recipient>]
            .iter()
            .map(|r| r.as_ref()),
    )
    .expect("we provided a recipient");

    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(message.as_bytes())?;
    writer.finish()?;

    console::log!("📦 Encoding to Base64...");
    let result = BASE64.encode(&encrypted);

    console::log!(&format!("✅ Encryption completed: {} bytes", result.len()));
    Ok(result)
}

fn decrypt_message(
    private_key: &str,
    encrypted_message: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    console::log!("🔍 Parsing X25519 private key...");
    let identity: x25519::Identity = private_key.parse()?;

    console::log!("📦 Decoding encrypted message from Base64...");
    let encrypted_bytes = BASE64.decode(encrypted_message)?;

    console::log!("🔓 Decrypting message...");
    let decryptor = match Decryptor::new(&encrypted_bytes[..]) {
        Ok(decryptor) => decryptor,
        Err(e) => {
            console::error!(&format!("❌ Error creating decryptor: {:?}", e));
            return Ok(None);
        }
    };

    let mut decrypted = vec![];
    let mut reader = match decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity)) {
        Ok(reader) => reader,
        Err(e) => {
            console::error!(&format!("❌ Error decrypting message: {:?}", e));
            return Ok(None);
        }
    };

    if let Err(e) = reader.read_to_end(&mut decrypted) {
        console::error!(&format!("❌ Error reading decrypted data: {:?}", e));
        return Ok(None);
    }

    console::log!("📝 Converting decrypted bytes to string...");
    let result = String::from_utf8(decrypted)?;

    console::log!(&format!("✅ Decryption completed: {} chars", result.len()));
    Ok(Some(result))
}

fn generate_public_key_from_private(
    private_key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    console::log!("🔍 Parsing X25519 private key...");
    let identity: x25519::Identity = private_key.parse()?;

    console::log!("🔑 Generating public key from private key...");
    let recipient = identity.to_public();

    let public_key_str = recipient.to_string();

    console::log!("✅ Public key generation completed");
    Ok(public_key_str)
}

fn process_qr_data(data: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    console::log!("🔄 Processing QR data");
    console::log!(&format!("📊 Data length: {}", data.len()));
    console::log!(&format!(
        "🔍 Data preview: {}...",
        &data[..data.len().min(50)]
    ));

    if is_valid_age_public_key(data) {
        console::log!("🔑 Age public key recognized");
        Ok(("add_contact".to_string(), data.to_string()))
    } else if is_base64(data) && data.len() > 50 && data.len() < 2000 {
        console::log!("🔓 Encrypted message recognized");
        Ok(("decrypt_message".to_string(), data.to_string()))
    } else {
        console::log!("📄 Other data recognized");
        Ok(("show_dialog".to_string(), format!("Read data: {}", data)))
    }
}

fn is_valid_age_public_key(data: &str) -> bool {
    match data.parse::<x25519::Recipient>() {
        Ok(_) => {
            console::log!("✅ Valid age public key verified");
            true
        }
        Err(_) => {
            console::log!("❌ Invalid age public key");
            false
        }
    }
}

fn is_base64(s: &str) -> bool {
    match BASE64.decode(s) {
        Ok(_) => true,
        Err(_) => false,
    }
}
