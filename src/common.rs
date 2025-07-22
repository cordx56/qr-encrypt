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
    PrivateKeyExported {
        encrypted_private_key: String,
    },
    PublicKeyGenerated {
        public_key: String,
    },
    QrDataProcessed {
        event_type: String,
        event_data: String,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum MainMessage {
    GenerateKeyPair,
    Encrypt {
        public_key: String,
        data: String,
    },
    Decrypt {
        private_key: String,
        data: String,
    },
    ExportPrivateKey {
        recipient_public_key: String,
        private_key: String,
    },
    GeneratePublicKeyFromPrivate {
        private_key: String,
    },
    ProcessQrData {
        data: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RtcMessage {
    PublicKey { public_key: String },
    EncryptedData { encrypted_data: String },
}
