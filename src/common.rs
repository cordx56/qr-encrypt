use gloo::{console, dialogs::alert};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkerMessage {
    Ready,
    Generated {
        public_key: String,
        private_key: String,
    },
    Encrypted {
        encrypted_data: String,
    },
    Decrypted {
        decrypted_data: String,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum MainMessage {
    GenerateKeyPair,
    Encrypt { public_key: String, data: String },
    Decrypt { private_key: String, data: String },
}
