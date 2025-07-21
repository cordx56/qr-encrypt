#![no_main]

mod common;
use common::*;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo::console;
use rand::rngs::OsRng;
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

#[wasm_bindgen(start)]
pub fn worker_main() {
    console::log!("🔧 Worker started");
    let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

    let global_clone = global.clone();
    let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: MessageEvent| {
        let global_inner = global_clone.clone();
        console::log!("🔧 Worker message received");
        spawn_local(async move {
            match generate_key_pair_with_progress().await {
                Ok((public_key, private_key)) => {
                    let message = serde_wasm_bindgen::to_value(&WorkerMessage::Generated {
                        public_key,
                        private_key,
                    })
                    .unwrap();
                    console::log!("🔧 Sending message to main thread");
                    let _ = global_inner.post_message(&message);
                }
                Err(e) => {
                    console::error!("❌ Error generating key pair: {:?}", e.to_string());
                }
            }
        });
    }) as Box<dyn FnMut(_)>);

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
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
