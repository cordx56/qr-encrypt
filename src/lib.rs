mod common;
use common::*;
mod rtc;
use rtc::Connection;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo::{console, dialogs::alert};
use js_sys::Date;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    window, CanvasRenderingContext2d, CustomEvent, CustomEventInit, Event, HtmlCanvasElement,
    HtmlElement, HtmlTextAreaElement, MessageEvent, Storage, Worker,
};
use yew::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "signal_type")]
pub enum RtcSignalData {
    Offer { sdp_data: String },
    Answer { sdp_data: String },
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub content: String,
    pub is_sent: bool,  // true for sent, false for received
    pub timestamp: f64, // Use js timestamp
}

#[derive(Debug)]
pub enum Msg {
    LoadMyKeys,
    GenerateKeys,
    KeysLoaded(KeyPair, HashMap<String, String>),
    DrawQrCode(String),
    ShowQrReader,
    HideQrReader,
    StartCamera,
    ShowMessageDialog,
    HideMessageDialog,
    ShowEncryptedQr(String),
    HideEncryptedQr,
    ShowDeleteConfirm(String),
    ConfirmDeleteContact(String),
    CancelDeleteContact,
    DecryptMessage(String),
    ShowDialog(String),
    HideDialog,
    UpdateLoadingProgress(String, Option<u8>),
    SetLoading(bool),
    HandleCustomEvent(String, String),
    CopyPublicKey,
    CopyEncryptedMessage,
    ShowExportPrivateKeyDialog,
    HideExportPrivateKeyDialog,
    ExportPrivateKey(String),
    ShowPrivateKeyImportConfirm(String),
    HidePrivateKeyImportConfirm,
    ConfirmImportPrivateKey,
    CancelImportPrivateKey,
    ImportPrivateKeyWithPublicKey(String, String),
    ShowAddContactDialog(String),
    HideAddContactDialog,
    ConfirmAddContact(String),
    CancelAddContact,
    ShowResetConfirm,
    HideResetConfirm,
    ConfirmReset,
    CancelReset,
    // RTC関連のメッセージ
    ShowRtcDialog,
    HideRtcDialog,
    StartRtcConnection,
    ProcessRtcSignal(String),
    RtcConnectionEstablished,
    SendPublicKeyViaRtc,
    ShowChatView,
    HideChatView,
    SendChatMessage(String),
    UpdateChatInput(String),
    AddChatMessage(String, bool), // content, is_sent
    SetPeerPublicKey(String),
    DecryptReceivedMessage(String),   // encrypted_data
    SendEncryptedChatMessage(String), // encrypted_data
    ClearChatHistory,
}

#[derive(Clone)]
pub struct AppState {
    pub my_keys: Option<KeyPair>,
    pub contacts: HashMap<String, String>,
    pub qr_reader_visible: bool,
    pub camera_started: bool,
    pub dialog_message: Option<String>,
    pub my_public_key_qr: Option<String>,
    pub message_dialog_visible: bool,
    pub encrypted_qr_visible: bool,
    pub encrypted_qr_data: Option<String>,
    pub delete_confirm_visible: bool,
    pub delete_target: Option<String>,
    pub is_loading: bool,
    pub loading_message: String,
    pub loading_progress: Option<u8>,
    pub worker: Option<Worker>,
    pub export_private_key_dialog_visible: bool,
    pub private_key_import_confirm_visible: bool,
    pub private_key_to_import: Option<String>,
    pub add_contact_dialog_visible: bool,
    pub public_key_to_add: Option<String>,
    pub reset_confirm_visible: bool,
    // RTC関連の状態
    pub rtc_dialog_visible: bool,
    pub rtc_connection: Option<Connection>,
    pub rtc_connected: bool,
    pub rtc_peer_public_key: Option<String>,
    pub chat_visible: bool,
    pub chat_input: String,
    pub chat_messages: Vec<ChatMessage>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            my_keys: None,
            contacts: HashMap::new(),
            qr_reader_visible: false,
            camera_started: false,
            dialog_message: None,
            my_public_key_qr: None,
            message_dialog_visible: false,
            encrypted_qr_visible: false,
            encrypted_qr_data: None,
            delete_confirm_visible: false,
            delete_target: None,
            is_loading: true,
            loading_message: "Initializing application...".to_string(),
            loading_progress: Some(0),
            worker: None,
            export_private_key_dialog_visible: false,
            private_key_import_confirm_visible: false,
            private_key_to_import: None,
            add_contact_dialog_visible: false,
            public_key_to_add: None,
            reset_confirm_visible: false,
            rtc_dialog_visible: false,
            rtc_connection: None,
            rtc_connected: false,
            rtc_peer_public_key: None,
            chat_visible: false,
            chat_input: String::new(),
            chat_messages: Vec::new(),
        }
    }
}

pub struct App {
    state: AppState,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadMyKeys);

        setup_custom_event_listener(ctx.link().clone());

        Self {
            state: AppState {
                my_keys: None,
                contacts: HashMap::new(),
                qr_reader_visible: false,
                camera_started: false,
                dialog_message: None,
                my_public_key_qr: None,
                message_dialog_visible: false,
                encrypted_qr_visible: false,
                encrypted_qr_data: None,
                delete_confirm_visible: false,
                delete_target: None,
                is_loading: true,
                loading_message: "Initializing...".to_string(),
                loading_progress: Some(0),
                worker: None,
                export_private_key_dialog_visible: false,
                private_key_import_confirm_visible: false,
                private_key_to_import: None,
                add_contact_dialog_visible: false,
                public_key_to_add: None,
                reset_confirm_visible: false,
                rtc_dialog_visible: false,
                rtc_connection: None,
                rtc_connected: false,
                rtc_peer_public_key: None,
                chat_visible: false,
                chat_input: String::new(),
                chat_messages: Vec::new(),
            },
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadMyKeys => {
                console::log!("📨 LoadMyKeys message received");
                self.initialize_app(ctx);
                true
            }
            Msg::GenerateKeys => {
                console::log!("📨 GenerateKeys message received");
                self.generate_new_keys(ctx);
                true
            }
            Msg::KeysLoaded(keys, contacts) => {
                console::log!("📨 KeysLoaded message received");
                self.state.my_keys = Some(keys.clone());
                self.state.contacts = contacts;
                self.state.is_loading = false;

                ctx.link().send_message(Msg::DrawQrCode(keys.public_key));
                true
            }
            Msg::DrawQrCode(public_key) => {
                console::log!("📨 DrawQrCode message received");

                let closure = Closure::wrap(Box::new(move || {
                    console::log!("⏰ QR code delayed drawing started");
                    draw_qr_code_to_canvas(&public_key);
                }) as Box<dyn FnMut()>);

                if let Some(window) = window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        100,
                    );
                    closure.forget();
                }
                true
            }
            Msg::ShowQrReader => {
                console::log!("📨 ShowQrReader message received");
                self.state.qr_reader_visible = true;
                self.state.camera_started = false;
                true
            }
            Msg::StartCamera => {
                console::log!("📨 StartCamera message received");
                self.state.camera_started = true;
                start_qr_reader_js();
                true
            }
            Msg::HideQrReader => {
                console::log!("📨 HideQrReader message received");
                self.state.qr_reader_visible = false;
                self.state.camera_started = false;
                stop_qr_reader_js();
                true
            }
            Msg::ShowMessageDialog => {
                console::log!("📨 ShowMessageDialog message received");
                self.state.message_dialog_visible = true;
                true
            }
            Msg::HideMessageDialog => {
                console::log!("📨 HideMessageDialog message received");
                self.state.message_dialog_visible = false;
                true
            }
            Msg::ShowEncryptedQr(encrypted_data) => {
                console::log!("📨 ShowEncryptedQr message received");
                self.state.encrypted_qr_data = Some(encrypted_data.clone());
                self.state.encrypted_qr_visible = true;

                let encrypted_data_clone = encrypted_data.clone();
                let closure = Closure::wrap(Box::new(move || {
                    console::log!("⏰ Encrypted QR code delayed drawing started");
                    draw_encrypted_qr_code_to_canvas(&encrypted_data_clone);
                }) as Box<dyn FnMut()>);

                if let Some(window) = window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        300,
                    );
                    closure.forget();
                }
                true
            }
            Msg::HideEncryptedQr => {
                console::log!("📨 HideEncryptedQr メッセージ受信");
                self.state.encrypted_qr_visible = false;
                self.state.encrypted_qr_data = None;
                true
            }
            Msg::ShowDeleteConfirm(name) => {
                console::log!("📨 ShowDeleteConfirm message received");
                console::log!(&format!("❓ Delete confirmation displayed: {}", name));
                self.state.delete_target = Some(name);
                self.state.delete_confirm_visible = true;
                true
            }
            Msg::ConfirmDeleteContact(name) => {
                console::log!("📨 ConfirmDeleteContact message received");
                console::log!(&format!("👤 Contact deletion executed: {}", name));
                self.delete_contact(name);
                self.state.delete_confirm_visible = false;
                self.state.delete_target = None;
                true
            }
            Msg::CancelDeleteContact => {
                console::log!("📨 CancelDeleteContact message received");
                console::log!("❌ Deletion canceled");
                self.state.delete_confirm_visible = false;
                self.state.delete_target = None;
                true
            }
            Msg::DecryptMessage(encrypted_message) => {
                console::log!("📨 DecryptMessage message received");
                console::log!("🔓 Decryption started");
                self.decrypt_and_show_message(encrypted_message);
                true
            }
            Msg::ShowDialog(message) => {
                self.state.dialog_message = Some(message);
                true
            }
            Msg::HideDialog => {
                self.state.dialog_message = None;
                true
            }
            Msg::UpdateLoadingProgress(message, progress) => {
                self.state.loading_message = message;
                self.state.loading_progress = progress;
                true
            }
            Msg::SetLoading(is_loading) => {
                self.state.is_loading = is_loading;
                true
            }
            Msg::HandleCustomEvent(event_type, data) => {
                console::log!("📨 HandleCustomEvent message received");
                console::log!(&format!("🎯 Custom event: {}", event_type));
                match event_type.as_str() {
                    "process_qr_data" => {
                        process_qr_data(&data);
                    }
                    "add_contact" => {
                        ctx.link().send_message(Msg::ShowAddContactDialog(data));
                        ctx.link().send_message(Msg::HideQrReader);
                    }
                    "delete_contact" => {
                        if let Ok(name) = serde_json::from_str::<String>(&data) {
                            ctx.link().send_message(Msg::ShowDeleteConfirm(name));
                            ctx.link().send_message(Msg::HideQrReader);
                        }
                    }
                    "decrypt_message" => {
                        ctx.link().send_message(Msg::DecryptMessage(data));
                        ctx.link().send_message(Msg::HideQrReader);
                    }
                    "show_dialog" => {
                        ctx.link().send_message(Msg::ShowDialog(data));
                        ctx.link().send_message(Msg::HideQrReader);
                    }
                    "show_encrypted_qr" => {
                        ctx.link().send_message(Msg::ShowEncryptedQr(data));
                    }
                    "encrypted_message_ready" => {
                        // Check if this is for chat (RTC connection is active) or QR generation
                        if self.state.rtc_connected && self.state.chat_visible {
                            ctx.link().send_message(Msg::SendEncryptedChatMessage(data));
                        } else {
                            dispatch_custom_event("show_encrypted_qr", &data);
                        }
                    }
                    "decrypted_message_ready" => {
                        // Check if this is for chat (RTC connection is active) or other purposes
                        if self.state.rtc_connected && self.state.chat_visible {
                            // This is a decrypted chat message, add it to the chat
                            ctx.link().send_message(Msg::AddChatMessage(data, false));
                        } else {
                            dispatch_custom_event("show_dialog", &data);
                        }
                    }
                    "encrypt_message" => {
                        console::log!("🔐 Encrypt message request received");
                        if let Ok(encrypt_data) = serde_json::from_str::<serde_json::Value>(&data) {
                            if let (Some(contact), Some(message)) = (
                                encrypt_data["contact"].as_str(),
                                encrypt_data["message"].as_str(),
                            ) {
                                console::log!(&format!("📋 Encrypting for contact: {}", contact));

                                // 連絡先の公開鍵を取得
                                if let Some(public_key) = self.state.contacts.get(contact) {
                                    // 暗号化を実行
                                    if let Some(worker) = self.state.worker.clone() {
                                        match serde_wasm_bindgen::to_value(&MainMessage::Encrypt {
                                            public_key: public_key.clone(),
                                            data: message.to_string(),
                                        }) {
                                            Ok(encrypt_message) => {
                                                if let Err(e) =
                                                    worker.post_message(&encrypt_message)
                                                {
                                                    console::error!(&format!(
                                                        "❌ Failed to post encrypt message: {:?}",
                                                        e
                                                    ));
                                                    ctx.link().send_message(Msg::ShowDialog(
                                                        "Failed to send encryption request"
                                                            .to_string(),
                                                    ));
                                                }
                                            }
                                            Err(e) => {
                                                console::error!(&format!(
                                                    "❌ Failed to serialize encrypt message: {:?}",
                                                    e
                                                ));
                                                ctx.link().send_message(Msg::ShowDialog(
                                                    "Failed to prepare encryption request"
                                                        .to_string(),
                                                ));
                                            }
                                        }
                                    } else {
                                        console::error!("❌ Worker not available");
                                        ctx.link().send_message(Msg::ShowDialog(
                                            "Worker not available".to_string(),
                                        ));
                                    }
                                } else {
                                    console::error!(&format!("❌ Contact {} not found", contact));
                                    ctx.link().send_message(Msg::ShowDialog(format!(
                                        "Contact '{}' not found",
                                        contact
                                    )));
                                }
                            } else {
                                console::error!("❌ Invalid encrypt_message data format");
                                ctx.link().send_message(Msg::ShowDialog(
                                    "Invalid encryption request format".to_string(),
                                ));
                            }
                        }
                    }
                    "public_key_generated" => {
                        console::log!("🔑 Public key generated, completing import");
                        if let Some(ref private_key) = self.state.private_key_to_import {
                            ctx.link().send_message(Msg::ImportPrivateKeyWithPublicKey(
                                private_key.clone(),
                                data,
                            ));
                        }
                    }
                    "process_rtc_signal" => {
                        console::log!("📡 RTC signal processing requested");
                        ctx.link().send_message(Msg::ProcessRtcSignal(data));
                        ctx.link().send_message(Msg::HideQrReader);
                    }
                    _ => {}
                }
                true
            }
            Msg::CopyPublicKey => {
                console::log!("📨 CopyPublicKey message received");
                if let Some(ref my_keys) = self.state.my_keys {
                    let public_key = my_keys.public_key.clone();
                    let ctx_link = ctx.link().clone();

                    spawn_local(async move {
                        match copy_to_clipboard(&public_key).await {
                            Ok(_) => {
                                console::log!("✅ Public key copied to clipboard");
                                ctx_link.send_message(Msg::ShowDialog(
                                    "Public key copied to clipboard!".to_string(),
                                ));
                            }
                            Err(_) => {
                                console::error!("❌ Failed to copy public key");
                                ctx_link.send_message(Msg::ShowDialog(
                                    "Failed to copy to clipboard".to_string(),
                                ));
                            }
                        }
                    });
                }
                true
            }
            Msg::CopyEncryptedMessage => {
                console::log!("📨 CopyEncryptedMessage message received");
                if let Some(ref encrypted_data) = self.state.encrypted_qr_data {
                    let encrypted_data_clone = encrypted_data.clone();
                    let ctx_link = ctx.link().clone();

                    spawn_local(async move {
                        match copy_to_clipboard(&encrypted_data_clone).await {
                            Ok(_) => {
                                console::log!("✅ Encrypted message copied to clipboard");
                                ctx_link.send_message(Msg::ShowDialog(
                                    "Encrypted message copied to clipboard!".to_string(),
                                ));
                            }
                            Err(_) => {
                                console::error!("❌ Failed to copy encrypted message");
                                ctx_link.send_message(Msg::ShowDialog(
                                    "Failed to copy to clipboard".to_string(),
                                ));
                            }
                        }
                    });
                } else {
                    console::error!("❌ No encrypted message to copy");
                    dispatch_custom_event("show_dialog", "No encrypted message available to copy!");
                }
                true
            }
            Msg::ShowExportPrivateKeyDialog => {
                console::log!("📨 ShowExportPrivateKeyDialog message received");
                self.state.export_private_key_dialog_visible = true;
                true
            }
            Msg::HideExportPrivateKeyDialog => {
                console::log!("📨 HideExportPrivateKeyDialog message received");
                self.state.export_private_key_dialog_visible = false;
                true
            }
            Msg::ExportPrivateKey(recipient_name) => {
                console::log!("📨 ExportPrivateKey message received");
                if let Some(ref my_keys) = self.state.my_keys {
                    if let Some(recipient_public_key) = self.state.contacts.get(&recipient_name) {
                        if let Some(worker) = self.state.worker.clone() {
                            match serde_wasm_bindgen::to_value(&MainMessage::ExportPrivateKey {
                                recipient_public_key: recipient_public_key.clone(),
                                private_key: my_keys.private_key.clone(),
                            }) {
                                Ok(export_message) => {
                                    if let Err(e) = worker.post_message(&export_message) {
                                        console::error!(&format!(
                                            "❌ Failed to post export message: {:?}",
                                            e
                                        ));
                                        ctx.link().send_message(Msg::ShowDialog(
                                            "Failed to export private key".to_string(),
                                        ));
                                    }
                                }
                                Err(e) => {
                                    console::error!(&format!(
                                        "❌ Failed to serialize export message: {:?}",
                                        e
                                    ));
                                    ctx.link().send_message(Msg::ShowDialog(
                                        "Failed to prepare export request".to_string(),
                                    ));
                                }
                            }
                        }
                    } else {
                        ctx.link()
                            .send_message(Msg::ShowDialog("Contact not found".to_string()));
                    }
                } else {
                    ctx.link()
                        .send_message(Msg::ShowDialog("No private key available".to_string()));
                }
                self.state.export_private_key_dialog_visible = false;
                true
            }
            Msg::ShowPrivateKeyImportConfirm(private_key) => {
                console::log!("📨 ShowPrivateKeyImportConfirm message received");
                self.state.private_key_to_import = Some(private_key);
                self.state.private_key_import_confirm_visible = true;
                true
            }
            Msg::HidePrivateKeyImportConfirm => {
                console::log!("📨 HidePrivateKeyImportConfirm message received");
                self.state.private_key_import_confirm_visible = false;
                self.state.private_key_to_import = None;
                true
            }
            Msg::ConfirmImportPrivateKey => {
                console::log!("📨 ConfirmImportPrivateKey message received");
                if let Some(ref private_key) = self.state.private_key_to_import {
                    // workerに秘密鍵から公開鍵を生成するよう依頼
                    if let Some(worker) = self.state.worker.clone() {
                        match serde_wasm_bindgen::to_value(
                            &MainMessage::GeneratePublicKeyFromPrivate {
                                private_key: private_key.clone(),
                            },
                        ) {
                            Ok(generate_message) => {
                                if let Err(e) = worker.post_message(&generate_message) {
                                    console::error!(&format!(
                                        "❌ Failed to post generate public key message: {:?}",
                                        e
                                    ));
                                    ctx.link().send_message(Msg::ShowDialog(
                                        "Failed to generate public key".to_string(),
                                    ));
                                    self.state.private_key_import_confirm_visible = false;
                                    self.state.private_key_to_import = None;
                                }
                            }
                            Err(e) => {
                                console::error!(&format!(
                                    "❌ Failed to serialize generate public key message: {:?}",
                                    e
                                ));
                                ctx.link().send_message(Msg::ShowDialog(
                                    "Failed to prepare public key generation".to_string(),
                                ));
                                self.state.private_key_import_confirm_visible = false;
                                self.state.private_key_to_import = None;
                            }
                        }
                    } else {
                        console::error!("❌ Worker not available for public key generation");
                        ctx.link()
                            .send_message(Msg::ShowDialog("Worker not available".to_string()));
                        self.state.private_key_import_confirm_visible = false;
                        self.state.private_key_to_import = None;
                    }
                } else {
                    self.state.private_key_import_confirm_visible = false;
                    self.state.private_key_to_import = None;
                }
                true
            }
            Msg::CancelImportPrivateKey => {
                console::log!("📨 CancelImportPrivateKey message received");
                self.state.private_key_import_confirm_visible = false;
                self.state.private_key_to_import = None;
                true
            }
            Msg::ImportPrivateKeyWithPublicKey(private_key, public_key) => {
                console::log!("📨 ImportPrivateKeyWithPublicKey message received");
                // 新しい鍵ペアを保存
                let new_keys = KeyPair {
                    private_key,
                    public_key: public_key.clone(),
                };
                self.state.my_keys = Some(new_keys.clone());

                // 秘密鍵インポートダイアログを閉じる
                self.state.private_key_import_confirm_visible = false;
                self.state.private_key_to_import = None;

                // 新しい鍵ペアを保存
                spawn_local(async move {
                    save_my_keys(&new_keys.private_key, &new_keys.public_key).await;
                });

                // QRコードを更新
                ctx.link().send_message(Msg::DrawQrCode(public_key));
                ctx.link().send_message(Msg::ShowDialog(
                    "Private key imported and saved successfully!".to_string(),
                ));
                true
            }
            Msg::ShowAddContactDialog(public_key) => {
                console::log!("📨 ShowAddContactDialog message received");
                self.state.public_key_to_add = Some(public_key);
                self.state.add_contact_dialog_visible = true;
                true
            }
            Msg::HideAddContactDialog => {
                console::log!("📨 HideAddContactDialog message received");
                self.state.add_contact_dialog_visible = false;
                self.state.public_key_to_add = None;
                true
            }
            Msg::ConfirmAddContact(name) => {
                console::log!("📨 ConfirmAddContact message received");
                if let Some(ref public_key) = self.state.public_key_to_add {
                    if !name.trim().is_empty() {
                        self.add_contact(name, public_key.clone());
                        self.state.add_contact_dialog_visible = false;
                        self.state.public_key_to_add = None;
                    }
                }
                true
            }
            Msg::CancelAddContact => {
                console::log!("📨 CancelAddContact message received");
                self.state.add_contact_dialog_visible = false;
                self.state.public_key_to_add = None;
                true
            }
            Msg::ShowResetConfirm => {
                console::log!("📨 ShowResetConfirm message received");
                self.state.reset_confirm_visible = true;
                true
            }
            Msg::HideResetConfirm => {
                console::log!("📨 HideResetConfirm message received");
                self.state.reset_confirm_visible = false;
                true
            }
            Msg::ConfirmReset => {
                console::log!("📨 ConfirmReset message received");
                self.reset_all_data();
                self.state.reset_confirm_visible = false;
                true
            }
            Msg::CancelReset => {
                console::log!("📨 CancelReset message received");
                self.state.reset_confirm_visible = false;
                true
            }
            Msg::ShowRtcDialog => {
                self.state.rtc_dialog_visible = true;
                true
            }
            Msg::HideRtcDialog => {
                self.state.rtc_dialog_visible = false;
                true
            }
            Msg::StartRtcConnection => {
                console::log!("📨 StartRtcConnection message received");
                self.state.rtc_connected = false;
                self.state.rtc_peer_public_key = None;
                self.state.rtc_dialog_visible = false;

                // 実際のWebRTC接続を開始
                let connection = Connection::new();
                let ctx_link = ctx.link().clone();

                // 接続確立ハンドラを設定（ICE接続のみ）
                let ctx_link_for_ice = ctx_link.clone();
                let _ = connection.set_connection_established_handler(move || {
                    console::log!("🎉 WebRTC ICE connection established!");
                    ctx_link_for_ice.send_message(Msg::RtcConnectionEstablished);
                });

                // データチャネルオープンハンドラを設定（メッセージング準備完了）
                let ctx_link_for_data = ctx_link.clone();
                let _ = connection.set_data_channel_open_handler(move || {
                    console::log!("🎉 Data channel ready! Starting public key exchange");
                    ctx_link_for_data.send_message(Msg::SendPublicKeyViaRtc);
                });

                self.state.rtc_connection = Some(connection.clone());

                spawn_local(async move {
                    let mut conn = connection;
                    match conn
                        .start_connection(move |offer_sdp| {
                            let offer_data = RtcSignalData::Offer {
                                sdp_data: offer_sdp,
                            };

                            if let Ok(offer_json) = serde_json::to_string(&offer_data) {
                                ctx_link.send_message(Msg::ShowEncryptedQr(offer_json));
                            }
                        })
                        .await
                    {
                        Ok(_) => {
                            console::log!("✅ RTC connection started successfully");
                        }
                        Err(e) => {
                            console::error!(&format!("❌ Failed to start RTC connection: {:?}", e));
                        }
                    }
                });

                true
            }
            Msg::ProcessRtcSignal(sdp_data) => {
                console::log!("📨 ProcessRtcSignal message received");

                if let Ok(signal_data) = serde_json::from_str::<RtcSignalData>(&sdp_data) {
                    console::log!("📡 Processing RTC signal");

                    match signal_data {
                        RtcSignalData::Offer { sdp_data } => {
                            // rtc.rsの実装を使用してOfferを処理
                            let connection = Connection::new();
                            let ctx_link = ctx.link().clone();
                            let offer_sdp = sdp_data.clone();

                            // 接続確立ハンドラを設定（ICE接続のみ）
                            let ctx_link_for_ice = ctx_link.clone();
                            let _ = connection.set_connection_established_handler(move || {
                                console::log!(
                                    "🎉 WebRTC ICE connection established (Answer side)!"
                                );
                                ctx_link_for_ice.send_message(Msg::RtcConnectionEstablished);
                            });

                            // データチャネルオープンハンドラを設定（メッセージング準備完了）
                            let ctx_link_for_data = ctx_link.clone();
                            let _ = connection.set_data_channel_open_handler(move || {
                                 console::log!("🎉 Data channel ready (Answer side)! Starting public key exchange");
                                 ctx_link_for_data.send_message(Msg::SendPublicKeyViaRtc);
                             });

                            // 接続をstateに保存
                            self.state.rtc_connection = Some(connection.clone());

                            spawn_local(async move {
                                let mut conn = connection;
                                match conn
                                    .recv_offer(offer_sdp, move |answer_sdp| {
                                        let answer_data = RtcSignalData::Answer {
                                            sdp_data: answer_sdp,
                                        };

                                        if let Ok(answer_json) = serde_json::to_string(&answer_data)
                                        {
                                            ctx_link
                                                .send_message(Msg::ShowEncryptedQr(answer_json));
                                        }
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        console::log!("✅ Offer processed successfully");
                                    }
                                    Err(e) => {
                                        console::error!(&format!(
                                            "❌ Failed to process offer: {:?}",
                                            e
                                        ));
                                    }
                                }
                            });
                        }
                        RtcSignalData::Answer { sdp_data } => {
                            // Answerを受信したので既存の接続でAnswer処理
                            if let Some(ref connection) = self.state.rtc_connection {
                                let connection_clone = connection.clone();
                                let answer_sdp = sdp_data.clone();

                                spawn_local(async move {
                                    let mut conn = connection_clone;
                                    match conn.recv_answer(answer_sdp).await {
                                        Ok(_) => {
                                            console::log!("✅ Answer processed successfully");
                                        }
                                        Err(e) => {
                                            console::error!(&format!(
                                                "❌ Failed to process answer: {:?}",
                                                e
                                            ));
                                        }
                                    }
                                });
                            } else {
                                console::error!("❌ No RTC connection to process answer");
                            }
                        }
                    }
                } else {
                    console::error!("❌ Failed to parse RTC signal data");
                    ctx.link()
                        .send_message(Msg::ShowDialog("Invalid RTC signal format".to_string()));
                }
                true
            }
            Msg::RtcConnectionEstablished => {
                console::log!(
                    "📨 RtcConnectionEstablished message received - ICE connection ready"
                );
                self.state.rtc_connected = true;
                self.state.rtc_dialog_visible = false;

                // RTC接続確立時に自動でチャット画面を表示
                self.state.chat_visible = true;

                // 注意: 公開鍵交換はデータチャネルオープン時に自動開始される

                true
            }
            Msg::SendPublicKeyViaRtc => {
                console::log!("📨 SendPublicKeyViaRtc message received");

                if let Some(ref my_keys) = self.state.my_keys {
                    if let Some(ref connection) = self.state.rtc_connection {
                        // RTC経由で公開鍵メッセージを送信
                        let public_key_message = RtcMessage::PublicKey {
                            public_key: my_keys.public_key.clone(),
                        };

                        console::log!("🔑 Sending public key via RTC");

                        let _connection_clone = connection.clone();
                        let ctx_link = ctx.link().clone();

                        spawn_local(async move {
                            let mut conn = _connection_clone;

                            // データハンドラを設定して受信メッセージを処理
                            let ctx_link_clone = ctx_link.clone();
                            if let Err(e) = conn.set_data_handler(move |data| {
                                console::log!(&format!("📨 Received RTC data: {}", data));

                                if let Ok(message) = serde_json::from_str::<RtcMessage>(&data) {
                                    match message {
                                        RtcMessage::PublicKey { public_key } => {
                                            console::log!("🔑 Received peer public key");
                                            ctx_link_clone.send_message(Msg::SetPeerPublicKey(
                                                public_key.clone(),
                                            ));
                                        }
                                        RtcMessage::EncryptedData { encrypted_data } => {
                                            console::log!(
                                                "🔓 Received encrypted data (chat message)"
                                            );
                                            // Decrypt received message before adding to chat
                                            ctx_link_clone.send_message(
                                                Msg::DecryptReceivedMessage(encrypted_data),
                                            );
                                        }
                                    }
                                }
                            }) {
                                console::error!(&format!("❌ Failed to set data handler: {:?}", e));
                            }

                            // データチャネルの状態を確認してからメッセージを送信
                            console::log!("⏳ Attempting to send public key via RTC...");
                            match conn.send_message(&public_key_message) {
                                Ok(_) => {
                                    console::log!("✅ Public key sent successfully");
                                }
                                Err(e) => {
                                    console::error!(&format!(
                                        "❌ Failed to send public key: {:?}",
                                        e
                                    ));
                                    ctx_link.send_message(Msg::ShowDialog(format!(
                                         "Failed to send public key: {}. Please try again after the connection is fully established.",
                                         e.as_string().unwrap_or_else(|| "Unknown error".to_string())
                                     )));
                                }
                            }
                        });
                    } else {
                        console::error!("❌ No RTC connection available");
                        ctx.link().send_message(Msg::ShowDialog(
                            "No RTC connection available".to_string(),
                        ));
                    }
                } else {
                    ctx.link().send_message(Msg::ShowDialog(
                        "No public key available to send".to_string(),
                    ));
                }
                true
            }
            Msg::ShowChatView => {
                self.state.chat_visible = true;
                true
            }
            Msg::HideChatView => {
                self.state.chat_visible = false;
                true
            }
            Msg::SendChatMessage(message) => {
                console::log!("📨 SendChatMessage:", &message);
                if let Some(ref connection) = self.state.rtc_connection {
                    if let Some(ref peer_public_key) = self.state.rtc_peer_public_key {
                        // メッセージを暗号化してから送信
                        if let Some(ref worker) = self.state.worker {
                            let worker_clone = worker.clone();
                            let message_clone = message.clone();
                            let peer_key_clone = peer_public_key.clone();
                            let _connection_clone = connection.clone();
                            let ctx_link = ctx.link().clone();

                            spawn_local(async move {
                                let encrypt_message = MainMessage::Encrypt {
                                    public_key: peer_key_clone,
                                    data: message_clone.clone(),
                                };

                                if let Ok(js_message) =
                                    serde_wasm_bindgen::to_value(&encrypt_message)
                                {
                                    worker_clone.post_message(&js_message).unwrap_or_else(|e| {
                                        console::error!(&format!(
                                            "❌ Failed to post encrypt message: {:?}",
                                            e
                                        ));
                                    });
                                }

                                // 送信済みメッセージをチャットに追加（暗号化前の平文で表示）
                                ctx_link.send_message(Msg::AddChatMessage(message_clone, true));
                            });
                        }
                        self.state.chat_input.clear();
                    } else {
                        console::error!("❌ No peer public key available for encryption");
                        ctx.link().send_message(Msg::ShowDialog("Peer public key not available. Please ensure the connection is fully established.".to_string()));
                    }
                }
                true
            }
            Msg::UpdateChatInput(input) => {
                self.state.chat_input = input;
                true
            }
            Msg::AddChatMessage(content, is_sent) => {
                let timestamp = Date::now();
                let message = ChatMessage {
                    content,
                    is_sent,
                    timestamp,
                };
                self.state.chat_messages.push(message);

                // Auto-scroll to bottom after adding message
                spawn_local(async move {
                    if let Some(window) = window() {
                        if let Some(document) = window.document() {
                            if let Some(messages_container) =
                                document.get_element_by_id("chat-messages")
                            {
                                let messages_element: HtmlElement =
                                    messages_container.unchecked_into();
                                messages_element.set_scroll_top(messages_element.scroll_height());
                            }
                        }
                    }
                });

                true
            }
            Msg::SetPeerPublicKey(public_key) => {
                self.state.rtc_peer_public_key = Some(public_key);
                true
            }
            Msg::DecryptReceivedMessage(encrypted_data) => {
                console::log!("🔓 Decrypting received message");
                if let Some(ref my_keys) = self.state.my_keys {
                    if let Some(ref worker) = self.state.worker {
                        let worker_clone = worker.clone();
                        let private_key = my_keys.private_key.clone();

                        spawn_local(async move {
                            let decrypt_message = MainMessage::Decrypt {
                                private_key,
                                data: encrypted_data,
                            };

                            if let Ok(js_message) = serde_wasm_bindgen::to_value(&decrypt_message) {
                                worker_clone.post_message(&js_message).unwrap_or_else(|e| {
                                    console::error!(&format!(
                                        "❌ Failed to post decrypt message: {:?}",
                                        e
                                    ));
                                });
                            }
                        });
                    }
                }
                true
            }
            Msg::SendEncryptedChatMessage(encrypted_data) => {
                console::log!("📨 Sending encrypted chat message");
                if let Some(ref connection) = self.state.rtc_connection {
                    let rtc_message = RtcMessage::EncryptedData { encrypted_data };
                    let mut conn = connection.clone();
                    match conn.send_message(&rtc_message) {
                        Ok(_) => {
                            console::log!("✅ Encrypted chat message sent successfully");
                        }
                        Err(e) => {
                            console::error!(&format!(
                                "❌ Failed to send encrypted chat message: {:?}",
                                e
                            ));
                        }
                    }
                }
                true
            }
            Msg::ClearChatHistory => {
                self.state.chat_messages.clear();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        console::log!("🖼️ View function called");
        console::log!(&format!(
            "🔑 Key state: {}",
            if self.state.my_keys.is_some() {
                "exists"
            } else {
                "none"
            }
        ));
        console::log!(&format!("⏳ Loading state: {}", self.state.is_loading));

        html! {
            <div class="app">
                <h1>{"qr-encrypt"}</h1>

                if self.state.is_loading {
                    { self.render_loading_screen() }
                } else if self.state.chat_visible {
                    { self.render_chat_view(ctx) }
                } else if let Some(ref _keys) = self.state.my_keys {
                    { self.render_main_view(ctx) }
                } else {
                    { self.render_loading_view() }
                }

                if self.state.rtc_dialog_visible && !self.state.is_loading {
                    { self.render_rtc_dialog(ctx) }
                }

                if self.state.qr_reader_visible && !self.state.is_loading {
                    { self.render_qr_reader(ctx) }
                }

                if self.state.message_dialog_visible && !self.state.is_loading {
                    { self.render_message_dialog(ctx) }
                }

                if self.state.encrypted_qr_visible && !self.state.is_loading {
                    { self.render_encrypted_qr_dialog(ctx) }
                }

                if self.state.delete_confirm_visible && !self.state.is_loading {
                    { self.render_delete_confirm_dialog(ctx) }
                }

                if self.state.export_private_key_dialog_visible && !self.state.is_loading {
                    { self.render_export_private_key_dialog(ctx) }
                }

                if self.state.private_key_import_confirm_visible && !self.state.is_loading {
                    { self.render_private_key_import_confirm_dialog(ctx) }
                }

                if self.state.add_contact_dialog_visible && !self.state.is_loading {
                    { self.render_add_contact_dialog(ctx) }
                }

                if self.state.reset_confirm_visible && !self.state.is_loading {
                    { self.render_reset_confirm_dialog(ctx) }
                }

                if let Some(ref message) = self.state.dialog_message {
                    if !self.state.is_loading {
                        { self.render_dialog(ctx, message) }
                    }
                }

                <p style="text-align: center; margin-top: 20px;">
                    <a href="https://github.com/cordx56/qr-encrypt">{"GitHub repository"}</a>
                </p>
            </div>
        }
    }
}

impl App {
    fn setup_worker(&mut self, ctx: &Context<Self>) {
        let link = ctx.link().clone();

        // ワーカー初期化のエラーハンドリングを改善
        let worker = match Worker::new("./worker_loader.js") {
            Ok(worker) => worker,
            Err(e) => {
                error_report(&format!("❌ Failed to create worker: {:?}", e));
                return;
            }
        };

        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            let link_clone = link.clone();
            match serde_wasm_bindgen::from_value::<WorkerMessage>(event.data()) {
                Ok(WorkerMessage::Ready) => {
                    console::log!("✅ Worker ready");
                    link.send_message(Msg::UpdateLoadingProgress(
                        "Checking saved keys...".to_string(),
                        Some(10),
                    ));

                    spawn_local(async move {
                        console::log!("💾 localStorage check started");

                        link_clone.send_message(Msg::UpdateLoadingProgress(
                            "Searching for keys...".to_string(),
                            Some(20),
                        ));

                        if let Some(keys) = load_my_keys().await {
                            console::log!("✅ Existing keys found");

                            link_clone.send_message(Msg::UpdateLoadingProgress(
                                "Keys loaded successfully".to_string(),
                                Some(90),
                            ));

                            let contacts = load_contacts().await;

                            link_clone.send_message(Msg::UpdateLoadingProgress(
                                "Application ready".to_string(),
                                Some(100),
                            ));

                            link_clone.send_message(Msg::SetLoading(false));
                            link_clone.send_message(Msg::KeysLoaded(keys, contacts));
                        } else {
                            console::log!("⚪ No existing keys found");
                            link_clone.send_message(Msg::UpdateLoadingProgress(
                                "Generating new keys...".to_string(),
                                Some(30),
                            ));
                            link_clone.send_message(Msg::GenerateKeys);
                        }
                    });
                }
                Ok(WorkerMessage::Generated {
                    public_key,
                    private_key,
                }) => {
                    console::log!("✅ X25519 key generation completed");

                    let public_key_clone = public_key.clone();
                    spawn_local(async move {
                        link_clone.send_message(Msg::UpdateLoadingProgress(
                            "Saving keys...".to_string(),
                            Some(85),
                        ));

                        save_my_keys(&private_key, &public_key).await;
                        console::log!("✅ Keys saved successfully");

                        let keys = KeyPair {
                            public_key: public_key.clone(),
                            private_key,
                        };

                        let contacts = HashMap::new();
                        link_clone.send_message(Msg::UpdateLoadingProgress(
                            "Application ready".to_string(),
                            Some(100),
                        ));

                        link_clone.send_message(Msg::SetLoading(false));
                        link_clone.send_message(Msg::KeysLoaded(keys, contacts));

                        link_clone.send_message(Msg::DrawQrCode(public_key_clone));
                    });
                }
                Ok(WorkerMessage::Encrypted { encrypted_data }) => {
                    console::log!("✅ Encryption successful");
                    dispatch_custom_event("encrypted_message_ready", &encrypted_data);
                }
                Ok(WorkerMessage::Decrypted { decrypted_data }) => {
                    console::log!("✅ Decryption successful");
                    // Check if the decrypted data is a private key
                    if is_private_key_data(&decrypted_data) {
                        console::log!("🔑 Private key detected in decrypted data");
                        link.send_message(Msg::ShowPrivateKeyImportConfirm(decrypted_data));
                    } else {
                        dispatch_custom_event("decrypted_message_ready", &decrypted_data);
                    }
                }
                Ok(WorkerMessage::PrivateKeyExported {
                    encrypted_private_key,
                }) => {
                    console::log!("✅ Private key export successful");
                    dispatch_custom_event("show_encrypted_qr", &encrypted_private_key);
                }
                Ok(WorkerMessage::PublicKeyGenerated { public_key }) => {
                    console::log!("✅ Public key generation successful");
                    dispatch_custom_event("public_key_generated", &public_key);
                }
                Ok(WorkerMessage::QrDataProcessed {
                    event_type,
                    event_data,
                }) => {
                    console::log!("✅ QR data processed successfully");
                    dispatch_custom_event(&event_type, &event_data);
                }
                Ok(WorkerMessage::Error { message }) => {
                    error_report(&message);
                }
                Err(e) => {
                    error_report(&format!("❌ Key generation error: {}", e.to_string()));
                    spawn_local(async move {
                        link_clone.send_message(Msg::UpdateLoadingProgress(
                            "Key generation failed".to_string(),
                            Some(0),
                        ));
                        link_clone.send_message(Msg::SetLoading(false));
                    });
                }
            }
        }) as Box<dyn FnMut(_)>);
        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
        self.state.worker = Some(worker);
    }

    fn initialize_app(&mut self, ctx: &Context<Self>) {
        console::log!("🔧 Application initialization started");
        self.setup_worker(ctx);
    }

    fn generate_new_keys(&mut self, ctx: &Context<Self>) {
        console::log!("🔧 New key generation started");
        let link = ctx.link().clone();

        ctx.link().send_message(Msg::UpdateLoadingProgress(
            "Starting key generation...".to_string(),
            Some(40),
        ));

        if let Some(worker) = self.state.worker.clone() {
            spawn_local(async move {
                link.send_message(Msg::UpdateLoadingProgress(
                    "Generating X25519 key pair...".to_string(),
                    Some(50),
                ));

                // GenerateKeyPairメッセージを送信
                match serde_wasm_bindgen::to_value(&MainMessage::GenerateKeyPair) {
                    Ok(generate_message) => {
                        if let Err(e) = worker.post_message(&generate_message) {
                            error_report(&format!("❌ Failed to request key generation: {:?}", e));
                            link.send_message(Msg::SetLoading(false));
                        }
                    }
                    Err(e) => {
                        error_report(&format!(
                            "❌ Failed to serialize key generation request: {:?}",
                            e
                        ));
                        link.send_message(Msg::SetLoading(false));
                    }
                }
            });
        } else {
            error_report("❌ Worker not available for key generation");
            ctx.link().send_message(Msg::SetLoading(false));
        }
    }

    fn add_contact(&mut self, name: String, public_key: String) {
        self.state.contacts.insert(name.clone(), public_key.clone());
        let contacts_clone = self.state.contacts.clone();
        spawn_local(async move {
            save_contacts(&contacts_clone).await;
        });
    }

    fn delete_contact(&mut self, name: String) {
        self.state.contacts.remove(&name);
        let contacts_clone = self.state.contacts.clone();
        spawn_local(async move {
            save_contacts(&contacts_clone).await;
        });
    }

    fn reset_all_data(&mut self) {
        console::log!("🗑️ Resetting all data");

        // Clear localStorage
        if let Some(storage) = get_local_storage() {
            let _ = storage.remove_item("mySecretKey");
            let _ = storage.remove_item("myPublicKey");
            let _ = storage.remove_item("keys");
            console::log!("✅ localStorage cleared");
        }

        // Reload the page to restart the application
        if let Some(window) = window() {
            let _ = window.location().reload();
        }
    }

    fn decrypt_and_show_message(&mut self, encrypted_message: String) {
        if let Some(ref keys) = self.state.my_keys {
            let private_key = keys.private_key.clone();
            if let Some(worker) = self.state.worker.clone() {
                match serde_wasm_bindgen::to_value(&MainMessage::Decrypt {
                    private_key,
                    data: encrypted_message,
                }) {
                    Ok(decrypt_message) => {
                        if let Err(e) = worker.post_message(&decrypt_message) {
                            console::error!(&format!("❌ Failed to post decrypt message: {:?}", e));
                            error_report("Failed to send decryption request");
                        }
                    }
                    Err(e) => {
                        console::error!(&format!(
                            "❌ Failed to serialize decrypt message: {:?}",
                            e
                        ));
                        error_report("Failed to prepare decryption request");
                    }
                }
            } else {
                console::error!("❌ Worker not available for decryption");
                error_report("Worker not available for decryption");
            }
        } else {
            console::error!("❌ No private key available for decryption");
            error_report("No private key available");
        }
    }

    fn render_loading_screen(&self) -> Html {
        html! {
            <div class="loading-screen">
                <div class="loading-content">
                    <div class="loading-icon">
                        <div class="spinner"></div>
                    </div>
                    <h2 class="loading-title">{"initializing..."}</h2>
                    <p class="loading-message">{&self.state.loading_message}</p>

                    if let Some(progress) = self.state.loading_progress {
                        <div class="progress-container">
                            <div class="progress-bar">
                                <div class="progress-fill" style={format!("width: {}%", progress)}></div>
                            </div>
                            <div class="progress-text">{format!("{}%", progress)}</div>
                        </div>
                    }

                    <div class="loading-tips">
                        <p>{"When you start the application for the first time, it takes a little time to generate the encryption key."}</p>
                        <p>{"For security reasons, a X25519 key is generated."}</p>
                    </div>
                </div>
            </div>
        }
    }

    fn render_loading_view(&self) -> Html {
        html! {
            <div class="loading">
                <p>{"Generating keys..."}</p>
            </div>
        }
    }

    fn render_main_view(&self, ctx: &Context<Self>) -> Html {
        console::log!("🏠 Main view rendering started");
        let on_qr_read_click = ctx.link().callback(|_| Msg::ShowQrReader);
        let on_message_send_click = ctx.link().callback(|_| Msg::ShowMessageDialog);
        let on_export_private_key_click = ctx
            .link()
            .callback(|_: web_sys::MouseEvent| Msg::ShowExportPrivateKeyDialog);

        html! {
            <div class="main-view">
                <div class="my-qr-section">
                    <h2>{"Your public key"}</h2>
                    <div class="qr-code">
                        <canvas id="qr-canvas" width="300" height="300"
                                style="max-width: 100%; height: auto; display: block; margin: 0 auto; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);"></canvas>
                    </div>
                    <div style="margin-top: 10px;">
                        <button onclick={ctx.link().callback(|_| Msg::CopyPublicKey)}
                                class="copy-key-btn"
                                style="background-color: #3498db; color: white; border: none; padding: 12px 20px; border-radius: 5px; cursor: pointer; font-size: 1rem;">
                            {"Copy My Public Key"}
                        </button>
                    </div>
                </div>

                <div class="actions">
                    <button onclick={on_qr_read_click} class="read-qr-btn">
                        {"Read QR"}
                    </button>
                    <button onclick={on_message_send_click} class="send-message-btn" style="margin-left: 10px; background-color: #27ae60;">
                        {"Send message"}
                    </button>
                    <button onclick={ctx.link().callback(|_| Msg::ShowRtcDialog)} class="rtc-connect-btn" style="margin-left: 10px; background-color: #9b59b6;">
                        {"Chat"}
                    </button>
                </div>

                <div class="contacts">
                    <h3>{"Contacts"}</h3>
                    if self.state.contacts.is_empty() {
                        <p style="color: #7f8c8d; font-style: italic; text-align: center; padding: 20px;">
                            {"No contacts"}
                        </p>
                    } else {
                        <ul>
                            { for self.state.contacts.iter().map(|(name, _)| {
                                let name_clone = name.clone();
                                let on_delete = ctx.link().callback(move |_| Msg::ShowDeleteConfirm(name_clone.clone()));

                                html! {
                                   <li class="contact-item">
                                        <span class="contact-name">{name}</span>
                                        <button
                                            onclick={on_delete}
                                            class="delete-contact-btn"
                                            title={format!("Delete {}", name)}
                                            style="background-color: #e74c3c; color: white; border: none; padding: 5px 10px; border-radius: 4px; cursor: pointer; font-size: 12px; margin-left: 10px;">
                                            {"Delete"}
                                        </button>
                                    </li>
                                }
                            })}
                        </ul>
                    }
                </div>
                <div style="margin-top: 20px; text-align: center;">
                    <button onclick={on_export_private_key_click} class="export-private-key-btn" style="margin-left: 10px; background-color: #e67e22;">
                        {"Export Private Key"}
                    </button>
                    <button onclick={ctx.link().callback(|_| Msg::ShowResetConfirm)} class="reset-btn" style="margin-left: 10px; background-color: #e74c3c;">
                        {"Reset All Data"}
                    </button>
                </div>
            </div>
        }
    }

    fn render_qr_reader(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideQrReader);
        let on_start_camera = ctx.link().callback(|_| Msg::StartCamera);

        html! {
            <div class="qr-reader-overlay">
                <div class="qr-reader">
                    <h3>{"Read QR"}</h3>

                    if !self.state.camera_started {
                        <div style="text-align: center; margin: 20px 0;">
                            <p style="margin: 10px 0; color: #7f8c8d; font-size: 14px;">
                                {"📷 Click to start camera"}
                            </p>
                            <button onclick={on_start_camera}
                                    style="background-color: #27ae60; color: white; border: none; padding: 12px 24px; border-radius: 5px; cursor: pointer; font-size: 16px; margin: 10px 0;">
                                {"Start Camera"}
                            </button>
                        </div>
                    } else {
                        <div>
                            <p style="margin: 10px 0; color: #27ae60; font-size: 14px;">
                                {"📷 Camera is active - point QR code to camera"}
                            </p>
                            <div style="position: relative; display: inline-block; margin: 10px 0;">
                                <video id="qr-video" autoplay=true style="border: 2px solid #27ae60; border-radius: 8px;"></video>
                                <div id="scan-overlay" style="position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); width: 200px; height: 200px; border: 2px solid #27ae60; border-radius: 4px; pointer-events: none; background: rgba(39, 174, 96, 0.1);">
                                    <div style="position: absolute; top: -2px; left: -2px; width: 20px; height: 20px; border-top: 4px solid #27ae60; border-left: 4px solid #27ae60;"></div>
                                    <div style="position: absolute; top: -2px; right: -2px; width: 20px; height: 20px; border-top: 4px solid #27ae60; border-right: 4px solid #27ae60;"></div>
                                    <div style="position: absolute; bottom: -2px; left: -2px; width: 20px; height: 20px; border-bottom: 4px solid #27ae60; border-left: 4px solid #27ae60;"></div>
                                    <div style="position: absolute; bottom: -2px; right: -2px; width: 20px; height: 20px; border-bottom: 4px solid #27ae60; border-right: 4px solid #27ae60;"></div>
                                </div>
                            </div>
                            <div style="margin: 10px 0; text-align: center;">
                                <p id="scan-status" style="font-size: 14px; color: #27ae60; margin-bottom: 10px; font-weight: 500;">
                                    {"🔍 Scanning for QR codes..."}
                                </p>
                                <p style="font-size: 12px; color: #7f8c8d;">
                                    {"Hold the QR code steady in the green frame"}
                                </p>
                            </div>
                        </div>
                    }

                    <div class="manual-input-section">
                        <h4>{"Manual Input"}</h4>
                        <p style="font-size: 13px; color: #7f8c8d; text-align: center; margin-bottom: 15px;">
                            {"Paste public key or encrypted message below"}
                        </p>

                        <div class="input-group">
                            <label>{"Public Key or Encrypted Message:"}</label>
                            <div class="textarea-container">
                                <textarea id="manual-input"
                                         placeholder="Paste public key or encrypted message here..."
                                         style="min-height: 100px;">
                                </textarea>
                                <button onclick={ctx.link().callback(|_| {
                                    if let Some(window) = window() {
                                        if let Some(document) = window.document() {
                                            if let Some(textarea) = document.get_element_by_id("manual-input") {
                                                if let Ok(textarea_element) = textarea.dyn_into::<HtmlTextAreaElement>() {
                                                    let input_data = textarea_element.value();
                                                    if !input_data.trim().is_empty() {
                                                        process_qr_data(&input_data);
                                                        textarea_element.set_value("");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Msg::HideQrReader
                                })} class="input-btn" style="background-color: #e67e22;">
                                    {"Process Data"}
                                </button>
                            </div>
                        </div>
                    </div>

                    <div style="margin-top: 20px; text-align: center; border-top: 1px solid #ddd; padding-top: 15px;">
                        <button onclick={on_close} style="background-color: #95a5a6; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer;">
                            {"Close"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_message_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideMessageDialog);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 500px;">
                    <h3>{"Send message"}</h3>
                    <div style="margin: 20px 0;">
                        <label>{"Select destination:"}</label>
                        <select id="contact-select" style="width: 100%; padding: 8px; margin: 5px 0;">
                            <option value="">{"Select destination"}</option>
                            { for self.state.contacts.iter().map(|(name, _)| {
                                html! { <option value={name.clone()}>{name}</option> }
                            })}
                        </select>
                    </div>
                    <div style="margin: 20px 0;">
                        <label>{"Message:"}</label>
                        <textarea id="message-input"
                                placeholder="Enter the message to send"
                                style="width: 100%; height: 100px; padding: 8px; margin: 5px 0; resize: vertical;">
                        </textarea>
                    </div>
                    <div style="display: flex; justify-content: space-between;">
                        <button onclick={on_close} style="background-color: #95a5a6;">{"Cancel"}</button>
                        <button onclick={ctx.link().callback(|_| {
                            if let Some(window) = window() {
                                if let Some(document) = window.document() {
                                    if let Some(select_element) = document.get_element_by_id("contact-select") {
                                        if let Ok(select_value) = js_sys::Reflect::get(&select_element, &"value".into()) {
                                            if let Some(selected_contact) = select_value.as_string() {
                                                if let Some(textarea_element) = document.get_element_by_id("message-input") {
                                                    if let Ok(textarea_value) = js_sys::Reflect::get(&textarea_element, &"value".into()) {
                                                        if let Some(message) = textarea_value.as_string() {
                                                            if !selected_contact.is_empty() && !message.trim().is_empty() {
                                                                let encrypt_data = format!(
                                                                    r#"{{"contact":"{}","message":"{}"}}"#,
                                                                    selected_contact.replace("\"", "\\\""),
                                                                    message.replace("\"", "\\\"").replace("\n", "\\n")
                                                                );
                                                                dispatch_custom_event("encrypt_message", &encrypt_data);
                                                            } else {
                                                                let _ = js_sys::eval("alert('Please select a contact and enter a message.');");
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Msg::HideMessageDialog
                        })} style="background-color: #27ae60;">
                            {"Send"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_encrypted_qr_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideEncryptedQr);
        let on_copy = ctx.link().callback(|_| Msg::CopyEncryptedMessage);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 400px;">
                    <h3>{"Encrypted message"}</h3>
                    <div style="text-align: center; margin: 20px 0;">
                        <p>{"Please send this QR to the other party"}</p>
                        <canvas id="encrypted-qr-canvas" width="300" height="300"
                                style="max-width: 100%; height: auto; display: block; margin: 10px auto; border: 1px solid #ddd; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);"></canvas>
                    </div>
                    <div class="encrypted-dialog-buttons">
                        <button onclick={on_copy}
                                class="copy-encrypted-btn">
                            {"Copy Message"}
                        </button>
                        <button onclick={on_close}
                                class="close-dialog-btn">
                            {"Close"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_delete_confirm_dialog(&self, ctx: &Context<Self>) -> Html {
        let delete_target = self.state.delete_target.clone().unwrap_or_default();
        let on_confirm = ctx
            .link()
            .callback(move |_| Msg::ConfirmDeleteContact(delete_target.clone()));
        let on_cancel = ctx.link().callback(|_| Msg::CancelDeleteContact);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 300px;">
                    <h3>{"Delete contact?"}</h3>
                    <p style="margin: 15px 0;">
                        <strong>{&self.state.delete_target.as_ref().unwrap_or(&"Unknown".to_string())}</strong>
                        {" will no longer be able to communicate with this contact."}
                    </p>
                    <div style="display: flex; justify-content: space-between; gap: 10px; margin-top: 20px;">
                        <button onclick={on_cancel} style="background-color: #95a5a6; flex: 1;">{"Cancel"}</button>
                        <button onclick={on_confirm} style="background-color: #e74c3c; flex: 1;">{"Delete"}</button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_export_private_key_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideExportPrivateKeyDialog);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 500px;">
                    <h3>{"Export Private Key"}</h3>
                    <p style="margin: 15px 0; color: #e74c3c; font-weight: bold;">
                        {"⚠️ Warning: This will send your private key to the selected contact. Only do this if you trust them completely."}
                    </p>
                    <div style="margin: 20px 0;">
                        <label>{"Select recipient:"}</label>
                        <select id="export-recipient-select" style="width: 100%; padding: 8px; margin: 5px 0;">
                            <option value="">{"Select recipient"}</option>
                            { for self.state.contacts.iter().map(|(name, _)| {
                                html! { <option value={name.clone()}>{name}</option> }
                            })}
                        </select>
                    </div>
                    <div style="display: flex; justify-content: space-between;">
                        <button onclick={on_close} style="background-color: #95a5a6;">{"Cancel"}</button>
                        <button onclick={ctx.link().callback(|_| {
                            if let Some(window) = window() {
                                if let Some(document) = window.document() {
                                    if let Some(select_element) = document.get_element_by_id("export-recipient-select") {
                                        if let Ok(select_value) = js_sys::Reflect::get(&select_element, &"value".into()) {
                                            if let Some(selected_recipient) = select_value.as_string() {
                                                if !selected_recipient.is_empty() {
                                                    return Msg::ExportPrivateKey(selected_recipient);
                                                } else {
                                                    let _ = js_sys::eval("alert('Please select a recipient.');");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Msg::HideExportPrivateKeyDialog
                        })} style="background-color: #e67e22;">
                            {"Export"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_private_key_import_confirm_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_confirm = ctx.link().callback(|_| Msg::ConfirmImportPrivateKey);
        let on_cancel = ctx.link().callback(|_| Msg::CancelImportPrivateKey);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 400px;">
                    <h3>{"Import Private Key"}</h3>
                    <p style="margin: 15px 0;">
                        {"You have received a private key. Do you want to import it?"}
                    </p>
                    <p style="margin: 15px 0; color: #e67e22; font-weight: bold;">
                        {"⚠️ Warning: This will replace your current private key. Make sure you trust the sender."}
                    </p>
                    <div style="display: flex; justify-content: space-between; gap: 10px; margin-top: 20px;">
                        <button onclick={on_cancel} style="background-color: #95a5a6; flex: 1;">{"Cancel"}</button>
                        <button onclick={on_confirm} style="background-color: #27ae60; flex: 1;">{"Import"}</button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_add_contact_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_cancel = ctx.link().callback(|_| Msg::CancelAddContact);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 400px;">
                    <h3>{"Add Contact"}</h3>
                    <p style="margin: 15px 0;">
                        {"Please enter a name for this public key:"}
                    </p>
                    <div style="margin: 20px 0;">
                        <input type="text"
                               id="contact-name-input"
                               placeholder="Enter contact name"
                               style="width: 100%; padding: 8px; margin: 5px 0; border: 1px solid #ddd; border-radius: 4px; font-size: 14px;"
                               />
                    </div>
                    <div style="display: flex; justify-content: space-between; gap: 10px; margin-top: 20px;">
                        <button onclick={on_cancel} style="background-color: #95a5a6; flex: 1;">{"Cancel"}</button>
                        <button onclick={ctx.link().callback(|_| {
                            if let Some(window) = window() {
                                if let Some(document) = window.document() {
                                    if let Some(input_element) = document.get_element_by_id("contact-name-input") {
                                        if let Ok(input_value) = js_sys::Reflect::get(&input_element, &"value".into()) {
                                            if let Some(name) = input_value.as_string() {
                                                if !name.trim().is_empty() {
                                                    return Msg::ConfirmAddContact(name);
                                                } else {
                                                    let _ = js_sys::eval("alert('Please enter a contact name.');");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Msg::HideAddContactDialog
                        })} style="background-color: #27ae60; flex: 1;">
                            {"Add"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_reset_confirm_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_confirm = ctx.link().callback(|_| Msg::ConfirmReset);
        let on_cancel = ctx.link().callback(|_| Msg::CancelReset);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 400px;">
                    <h3>{"Reset All Data"}</h3>
                    <p style="margin: 15px 0; color: #e74c3c; font-weight: bold;">
                        {"⚠️ Warning: This will delete all your data including:"}
                    </p>
                    <ul style="text-align: left; margin: 15px 0; color: #34495e;">
                        <li>{"Your private and public keys"}</li>
                        <li>{"All saved contacts"}</li>
                        <li>{"All locally stored data"}</li>
                    </ul>
                    <p style="margin: 15px 0; color: #e74c3c;">
                        {"This action cannot be undone. Are you sure?"}
                    </p>
                    <div style="display: flex; justify-content: space-between; gap: 10px; margin-top: 20px;">
                        <button onclick={on_cancel} style="background-color: #95a5a6; flex: 1;">{"Cancel"}</button>
                        <button onclick={on_confirm} style="background-color: #e74c3c; flex: 1;">{"Reset"}</button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_dialog(&self, ctx: &Context<Self>, message: &str) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideDialog);

        html! {
            <div class="dialog-overlay">
                <div class="dialog">
                    <p>{message}</p>
                    <button onclick={on_close}>{"OK"}</button>
                </div>
            </div>
        }
    }

    fn render_rtc_dialog(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideRtcDialog);
        let on_generate_offer = ctx.link().callback(|_| Msg::StartRtcConnection);

        html! {
            <div class="dialog-overlay">
                <div class="dialog" style="max-width: 500px;">
                    <h3>{"WebRTC Connection Setup"}</h3>
                    <p style="margin: 15px 0;">
                        {"Start a secure peer-to-peer connection:"}
                    </p>

                    <div style="margin: 20px 0;">
                        <button onclick={on_generate_offer} style="background-color: #3498db; padding: 15px; font-size: 16px; width: 100%;">
                            {"Create Offer & Generate QR Code"}
                        </button>
                    </div>

                    <div style="margin: 20px 0;">
                        <h4>{"How it works:"}</h4>
                        <ol style="text-align: left; font-size: 14px; color: #666; padding-left: 20px;">
                            <li>{"Click \"Create Offer\" to generate a QR code"}</li>
                            <li>{"Share the QR code with the other party"}</li>
                            <li>{"They scan it and generate an answer QR code"}</li>
                            <li>{"Scan their answer QR code to complete the connection"}</li>
                            <li>{"Public keys will be exchanged automatically"}</li>
                        </ol>
                    </div>

                    if self.state.rtc_connected {
                        <div style="margin: 20px 0; padding: 15px; background-color: #d4edda; border-radius: 5px;">
                            <p style="color: #155724; margin: 0; font-weight: bold;">
                                {"✅ RTC Connection Established!"}
                            </p>
                            if let Some(ref peer_key) = self.state.rtc_peer_public_key {
                                <p style="color: #155724; margin: 5px 0 0 0; font-size: 12px;">
                                    {format!("Peer public key: {}...", &peer_key[..20])}
                                </p>
                            }
                        </div>
                    }

                    <div style="display: flex; justify-content: space-between; gap: 10px; margin-top: 20px;">
                        <button onclick={on_close} style="background-color: #95a5a6; flex: 1;">{"Close"}</button>
                        if self.state.rtc_connected {
                            <button onclick={ctx.link().callback(|_| Msg::SendPublicKeyViaRtc)} style="background-color: #e67e22; flex: 1;">
                                {"Send My Public Key"}
                            </button>
                        }
                    </div>
                </div>
            </div>
        }
    }

    fn render_chat_view(&self, ctx: &Context<Self>) -> Html {
        let on_input = ctx.link().callback(|e: web_sys::InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            Msg::UpdateChatInput(input.value())
        });

        let on_send = {
            let message = self.state.chat_input.clone();
            ctx.link().callback(move |_| {
                if !message.trim().is_empty() {
                    Msg::SendChatMessage(message.clone())
                } else {
                    Msg::UpdateChatInput(String::new())
                }
            })
        };

        let on_keydown = {
            let message = self.state.chat_input.clone();
            ctx.link().callback(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Enter" && !message.trim().is_empty() {
                    Msg::SendChatMessage(message.clone())
                } else {
                    Msg::UpdateChatInput(message.clone())
                }
            })
        };

        let on_back = ctx.link().callback(|_| Msg::HideChatView);

        html! {
            <div class="chat-container" style="max-width: 800px; margin: 0 auto; padding: 20px;">
                <div class="chat-header" style="display: flex; align-items: center; padding: 10px 0; border-bottom: 1px solid #ddd; margin-bottom: 20px;">
                    <button onclick={on_back} style="background-color: #95a5a6; margin-right: 15px; padding: 8px 15px;">
                        {"← Back"}
                    </button>
                    <h2 style="margin: 0; flex: 1;">{"Chat"}</h2>
                    <button onclick={ctx.link().callback(|_| Msg::ClearChatHistory)} style="background-color: #e74c3c; color: white; margin-right: 15px; padding: 8px 15px; border: none; border-radius: 4px; cursor: pointer;">
                        {"🗑️ Clear"}
                    </button>
                    <span style="color: #27ae60; font-weight: bold;">{"🔒"}</span>
                </div>

                <div id="chat-messages" class="chat-messages" style="min-height: 400px; max-height: 400px; overflow-y: auto; border: 1px solid #ddd; padding: 15px; margin-bottom: 20px; background-color: #f9f9f9;">
                    {
                        if self.state.chat_messages.is_empty() {
                            html! {
                                <p style="text-align: center; color: #666; font-style: italic;">
                                    {"Your messages are end-to-end encrypted via WebRTC"}
                                </p>
                            }
                        } else {
                            html! {
                                <>
                                    {
                                        self.state.chat_messages.iter().map(|msg| {
                                            let (message_style, alignment) = if msg.is_sent {
                                                ("background-color: #3498db; color: white; margin-left: auto; margin-right: 0;", "flex-end")
                                            } else {
                                                ("background-color: #ecf0f1; color: #2c3e50; margin-left: 0; margin-right: auto;", "flex-start")
                                            };

                                            html! {
                                                <div style={format!("display: flex; justify-content: {}; margin-bottom: 10px;", alignment)}>
                                                    <div style={format!("max-width: 70%; padding: 8px 12px; border-radius: 18px; word-wrap: break-word; {}", message_style)}>
                                                        {&msg.content}
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Html>()
                                    }
                                </>
                            }
                        }
                    }
                </div>

                <div class="chat-input" style="display: flex; gap: 10px;">
                    <input
                        type="text"
                        placeholder="Type your message..."
                        value={self.state.chat_input.clone()}
                        oninput={on_input}
                        onkeydown={on_keydown}
                        style="flex: 1; padding: 12px; border: 1px solid #ddd; border-radius: 5px; font-size: 16px;"
                    />
                    <button
                        onclick={on_send}
                        disabled={self.state.chat_input.trim().is_empty()}
                        style="padding: 12px 20px; background-color: #3498db; color: white; border: none; border-radius: 5px; font-size: 16px; cursor: pointer;"
                    >
                        {"Send"}
                    </button>
                </div>

                if let Some(ref peer_key) = self.state.rtc_peer_public_key {
                    <div style="margin-top: 15px; padding: 10px; background-color: #d4edda; border-radius: 5px; font-size: 12px;">
                        <strong>{"Connected to: "}</strong>
                        <span style="font-family: monospace;">{format!("{}...", &peer_key[..20])}</span>
                    </div>
                }
            </div>
        }
    }
}

fn setup_custom_event_listener(link: yew::html::Scope<App>) {
    if let Some(window) = window() {
        let document = window.document().unwrap();

        let link_clone = link.clone();
        let closure = Closure::wrap(Box::new(move |event: Event| {
            console::log!("🎯 Custom event received");

            if let Some(custom_event) = event.dyn_ref::<CustomEvent>() {
                if let Some(detail) = custom_event.detail().as_string() {
                    console::log!(&format!("📨 Event detail: {}", detail));

                    if let Ok(data) = serde_json::from_str::<(String, String)>(&detail) {
                        let (event_type, data) = data;
                        link_clone.send_message(Msg::HandleCustomEvent(event_type, data));
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        let _ = document
            .add_event_listener_with_callback("qr_app_event", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

fn dispatch_custom_event(event_type: &str, data: &str) {
    if let Some(window) = window() {
        if let Some(document) = window.document() {
            let event_data = serde_json::to_string(&(event_type.to_string(), data.to_string()))
                .unwrap_or_default();

            let custom_event_init = CustomEventInit::new();
            custom_event_init.set_detail(&JsValue::from_str(&event_data));

            if let Ok(custom_event) =
                CustomEvent::new_with_event_init_dict("qr_app_event", &custom_event_init)
            {
                let _ = document.dispatch_event(&custom_event);
            }
        }
    }
}

fn start_qr_reader_js() {
    if let Some(window) = window() {
        if let Ok(start_function) = js_sys::Reflect::get(&window, &"startQrReader".into()) {
            if start_function.is_function() {
                let _ = js_sys::Function::from(start_function).call0(&window);
            }
        }
    }
}

fn stop_qr_reader_js() {
    if let Some(window) = window() {
        if let Ok(stop_function) = js_sys::Reflect::get(&window, &"stopQrReader".into()) {
            if stop_function.is_function() {
                let _ = js_sys::Function::from(stop_function).call0(&window);
            }
        }
    }
}

fn draw_qr_code_to_canvas(data: &str) {
    console::log!("🎨 QR code drawing started");
    console::log!(&format!("📊 Data length: {}", data.len()));
    console::log!(&format!("🔍 Data preview: {}", &data[..data.len().min(50)]));

    let window = match window() {
        Some(w) => w,
        None => {
            console::error!("❌ Failed to get window object");
            return;
        }
    };

    let document = match window.document() {
        Some(d) => d,
        None => {
            console::error!("❌ Failed to get document object");
            return;
        }
    };

    let canvas_element = match document.get_element_by_id("qr-canvas") {
        Some(el) => el,
        None => {
            console::error!("❌ Canvas element 'qr-canvas' not found");
            return;
        }
    };

    let canvas = match canvas_element.dyn_into::<HtmlCanvasElement>() {
        Ok(c) => c,
        Err(_) => {
            console::error!("❌ Failed to cast element to HtmlCanvasElement");
            return;
        }
    };

    let viewport_width = window
        .inner_width()
        .map(|w| w.as_f64().unwrap_or(800.0))
        .unwrap_or(800.0);
    let canvas_size = if viewport_width < 360.0 {
        300
    } else if viewport_width < 480.0 {
        300
    } else {
        300
    };

    canvas.set_width(canvas_size as u32);
    canvas.set_height(canvas_size as u32);

    console::log!(&format!(
        "📐 Canvas size set: {}x{}",
        canvas_size, canvas_size
    ));

    let context = match canvas.get_context("2d") {
        Ok(Some(ctx)) => match ctx.dyn_into::<CanvasRenderingContext2d>() {
            Ok(c) => c,
            Err(_) => {
                console::error!("❌ Failed to cast context to CanvasRenderingContext2d");
                return;
            }
        },
        Ok(None) => {
            console::error!("❌ Failed to get 2D context (null)");
            return;
        }
        Err(_) => {
            console::error!("❌ Failed to get 2D context");
            return;
        }
    };

    if let Ok(qr_code) = QrCode::new(data) {
        let modules = qr_code.width();
        console::log!(&format!("⚫ QR modules: {}x{}", modules, modules));

        let margin = 12.0;
        let available_size = canvas_size as f64 - (margin * 2.0);
        let cell_size = available_size / modules as f64;

        context.set_fill_style_str("white");
        context.fill_rect(0.0, 0.0, canvas_size as f64, canvas_size as f64);

        console::log!(&format!(
            "🎯 Drawing parameters: cell_size={:.2}, margin={:.2}",
            cell_size, margin
        ));

        context.set_fill_style_str("black");
        for y in 0..modules {
            for x in 0..modules {
                if qr_code[(x, y)] == qrcode::Color::Dark {
                    let draw_x = margin + (x as f64 * cell_size);
                    let draw_y = margin + (y as f64 * cell_size);
                    context.fill_rect(draw_x, draw_y, cell_size, cell_size);
                }
            }
        }

        console::log!("✅ QR code drawing completed");
    } else {
        console::error!("❌ QR code generation failed");

        context.set_fill_style_str("#ffebee");
        context.fill_rect(0.0, 0.0, canvas_size as f64, canvas_size as f64);
        context.set_fill_style_str("#c62828");
        context.set_font("16px Arial");
        context.set_text_align("center");
        let _ = context.fill_text(
            "QR code generation error",
            canvas_size as f64 / 2.0,
            canvas_size as f64 / 2.0,
        );
    }
}

// 暗号化されたメッセージのQRコード描画
fn draw_encrypted_qr_code_to_canvas(data: &str) {
    console::log!("🎨 Encrypted QR code drawing started");
    console::log!(&format!("📊 Encrypted message data length: {}", data.len()));
    console::log!(&format!(
        "🔑 Encrypted message preview: {}...",
        &data[..data.len().min(50)]
    ));

    let qr_code = match QrCode::new(data) {
        Ok(qr) => qr,
        Err(e) => {
            console::error!(&format!("❌ Encrypted QR code generation error: {:?}", e));
            return;
        }
    };

    console::log!("✅ Encrypted QR code generated successfully");
    let modules = qr_code.width();
    console::log!(&format!(
        "📐 Encrypted QR code size: {}x{}",
        modules, modules
    ));

    let window = match window() {
        Some(w) => w,
        None => {
            console::error!("❌ Failed to get window object for encrypted QR");
            return;
        }
    };

    let document = match window.document() {
        Some(d) => d,
        None => {
            console::error!("❌ Failed to get document object for encrypted QR");
            return;
        }
    };

    let canvas_element = match document.get_element_by_id("encrypted-qr-canvas") {
        Some(el) => el,
        None => {
            console::error!("❌ Canvas element 'encrypted-qr-canvas' not found");
            return;
        }
    };

    let canvas = match canvas_element.dyn_into::<HtmlCanvasElement>() {
        Ok(c) => c,
        Err(_) => {
            console::error!("❌ Failed to cast encrypted canvas element to HtmlCanvasElement");
            return;
        }
    };

    let viewport_width = window
        .inner_width()
        .map(|w| w.as_f64().unwrap_or(800.0))
        .unwrap_or(800.0);
    let canvas_size = if viewport_width < 360.0 {
        300
    } else if viewport_width < 480.0 {
        300
    } else {
        300
    };

    canvas.set_width(canvas_size);
    canvas.set_height(canvas_size);
    console::log!(&format!(
        "📐 Encrypted canvas size adjusted: {}x{}",
        canvas_size, canvas_size
    ));

    let context = match canvas.get_context("2d") {
        Ok(Some(ctx)) => match ctx.dyn_into::<CanvasRenderingContext2d>() {
            Ok(c) => c,
            Err(_) => {
                console::error!("❌ Failed to cast encrypted context to CanvasRenderingContext2d");
                return;
            }
        },
        Ok(None) => {
            console::error!("❌ Failed to get 2D context for encrypted canvas (null)");
            return;
        }
        Err(_) => {
            console::error!("❌ Failed to get 2D context for encrypted canvas");
            return;
        }
    };

    console::log!("🎨 2D context obtained successfully for encrypted QR");

    // マージンを設定（角丸で欠けるのを防ぐ）
    let margin = 15.0; // 15px のマージン（暗号化QRは少し大きめ）
    let available_size = canvas_size as f64 - (margin * 2.0);
    let cell_size = available_size / modules as f64;
    console::log!(&format!(
        "📏 Encrypted margin: {}px, available size: {}px, cell size: {}",
        margin, available_size, cell_size
    ));

    // 背景を白に
    context.set_fill_style_str("white");
    context.fill_rect(0.0, 0.0, canvas_size as f64, canvas_size as f64);
    console::log!("⚪ Encrypted background drawing completed");

    // QRコードを描画（マージンを考慮）
    context.set_fill_style_str("black");
    let mut dark_modules = 0;
    for y in 0..modules {
        for x in 0..modules {
            if qr_code[(x, y)] == qrcode::Color::Dark {
                dark_modules += 1;
                context.fill_rect(
                    margin + (x as f64 * cell_size),
                    margin + (y as f64 * cell_size),
                    cell_size,
                    cell_size,
                );
            }
        }
    }
    console::log!(&format!(
        "⚫ Encrypted QR code drawing completed - dark modules: {}",
        dark_modules
    ));
    console::log!("🎉 Encrypted QR code drawing completed successfully!");
}

// localStorage関連の関数
async fn save_my_keys(private_key: &str, public_key: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item("mySecretKey", private_key);
        let _ = storage.set_item("myPublicKey", public_key);
    }
}

async fn load_my_keys() -> Option<KeyPair> {
    if let Some(storage) = get_local_storage() {
        if let (Ok(Some(private_key)), Ok(Some(public_key))) = (
            storage.get_item("mySecretKey"),
            storage.get_item("myPublicKey"),
        ) {
            return Some(KeyPair {
                private_key,
                public_key,
            });
        }
    }
    None
}

async fn save_contacts(contacts: &HashMap<String, String>) {
    if let Some(storage) = get_local_storage() {
        if let Ok(json) = serde_json::to_string(contacts) {
            let _ = storage.set_item("keys", &json);
        }
    }
}

async fn load_contacts() -> HashMap<String, String> {
    if let Some(storage) = get_local_storage() {
        if let Ok(Some(json)) = storage.get_item("keys") {
            if let Ok(contacts) = serde_json::from_str(&json) {
                return contacts;
            }
        }
    }
    HashMap::new()
}

fn error_report(message: &str) {
    console::error!(message);
    alert(&message);
}

fn get_local_storage() -> Option<Storage> {
    window()?.local_storage().ok()?
}

#[wasm_bindgen]
pub fn process_qr_data(data: &str) {
    console::log!("🔄 Wasm processing QR data");
    console::log!(&format!("📊 Data length: {}", data.len()));
    console::log!(&format!(
        "🔍 Data preview: {}...",
        &data[..data.len().min(50)]
    ));

    // RTC信号データかチェック
    if let Ok(_signal_data) = serde_json::from_str::<RtcSignalData>(data) {
        console::log!("📡 RTC signal data recognized");
        dispatch_custom_event("process_rtc_signal", data);
    } else if is_valid_age_public_key(data) {
        console::log!("🔑 Age public key recognized");
        dispatch_custom_event("add_contact", &data);
    } else if is_private_key_data(data) {
        console::log!("🔑 Private key recognized");
        dispatch_custom_event("add_contact", &data);
    } else if is_base64(data) && data.len() > 50 && data.len() < 2000 {
        console::log!("🔓 Encrypted message recognized");
        dispatch_custom_event("decrypt_message", &data);
    } else {
        console::log!("📄 Other data recognized");
        dispatch_custom_event("show_dialog", &format!("Read data: {}", data));
    }
}

fn is_valid_age_public_key(data: &str) -> bool {
    use age::x25519;
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

fn is_private_key_data(data: &str) -> bool {
    data.parse::<age::x25519::Identity>().is_ok()
}

fn is_base64(s: &str) -> bool {
    match BASE64.decode(s) {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    use js_sys::Promise;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{window, Clipboard};

    let window = window().ok_or("No window object")?;
    let navigator = window.navigator();

    // Clipboard APIが利用可能かチェック
    let clipboard = js_sys::Reflect::get(&navigator, &"clipboard".into())?;
    if clipboard.is_undefined() {
        return Err(JsValue::from_str("Clipboard API not available"));
    }

    let clipboard: Clipboard = clipboard.unchecked_into();
    let promise: Promise = clipboard.write_text(text);

    JsFuture::from(promise).await?;
    Ok(())
}

#[wasm_bindgen(start)]
pub fn main() {
    yew::Renderer::<App>::new().render();
}
