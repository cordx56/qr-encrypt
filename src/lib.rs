use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo::console;
use qrcode::QrCode;
use rand::rngs::OsRng;
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::{pkcs1v15::Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    window, CanvasRenderingContext2d, CustomEvent, CustomEventInit, Event, HtmlCanvasElement,
    HtmlTextAreaElement, Storage,
};
use yew::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug)]
pub enum Msg {
    LoadMyKeys,
    GenerateKeys,
    KeysLoaded(KeyPair, HashMap<String, String>),
    DrawQrCode(String),
    ShowQrReader,
    HideQrReader,
    ShowMessageDialog,
    HideMessageDialog,
    ShowEncryptedQr(String),
    HideEncryptedQr,
    ShowDeleteConfirm(String),
    HideDeleteConfirm,
    ConfirmDelete,
    ConfirmDeleteContact(String),
    CancelDeleteContact,
    SetDialogVisible(bool),
    AddContact(String, String),
    DeleteContact(String),
    DecryptMessage(String),
    ShowDialog(String),
    HideDialog,
    UpdateLoadingProgress(String, Option<u8>),
    SetLoading(bool),
    HandleCustomEvent(String, String),
    CopyPublicKey,
    CopyEncryptedMessage,
}

#[derive(Clone, PartialEq)]
pub struct AppState {
    pub my_keys: Option<KeyPair>,
    pub contacts: HashMap<String, String>,
    pub qr_reader_visible: bool,
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            my_keys: None,
            contacts: HashMap::new(),
            qr_reader_visible: false,
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

                let public_key_clone = public_key.clone();
                let closure = Closure::wrap(Box::new(move || {
                    console::log!("⏰ QR code delayed drawing started");
                    draw_qr_code_to_canvas(&public_key_clone);
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

                start_qr_reader_js();
                true
            }
            Msg::HideQrReader => {
                console::log!("📨 HideQrReader message received");
                self.state.qr_reader_visible = false;
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
            Msg::AddContact(name, public_key) => {
                console::log!("📨 AddContact message received");
                console::log!(&format!("👤 Contact added: {}", name));
                self.add_contact(name, public_key);
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
                console::log!("📨 ShowDialog message received");
                console::log!(&format!(
                    "💬 Dialog displayed: {}",
                    &message[..message.len().min(50)]
                ));
                self.state.dialog_message = Some(message);
                true
            }
            Msg::HideDialog => {
                console::log!("📨 HideDialog message received");
                self.state.dialog_message = None;
                true
            }
            Msg::UpdateLoadingProgress(message, progress) => {
                console::log!("📨 UpdateLoadingProgress message received");
                console::log!(&format!("💾 Loading message updated: {}", message));
                self.state.loading_message = message;
                self.state.loading_progress = progress;
                true
            }
            Msg::SetLoading(is_loading) => {
                console::log!("📨 SetLoading message received");
                self.state.is_loading = is_loading;
                true
            }
            Msg::HandleCustomEvent(event_type, data) => {
                console::log!("📨 HandleCustomEvent message received");
                console::log!(&format!("🎯 Custom event: {}", event_type));
                match event_type.as_str() {
                    "add_contact" => {
                        if let Ok(contact_data) = serde_json::from_str::<(String, String)>(&data) {
                            ctx.link()
                                .send_message(Msg::AddContact(contact_data.0, contact_data.1));
                            ctx.link().send_message(Msg::HideQrReader);
                        }
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
                                    match encrypt_message_sync(public_key, message) {
                                        Ok(encrypted) => {
                                            console::log!("✅ Encryption successful");
                                            ctx.link()
                                                .send_message(Msg::ShowEncryptedQr(encrypted));
                                        }
                                        Err(e) => {
                                            console::error!(&format!(
                                                "❌ Encryption error: {:?}",
                                                e
                                            ));
                                            ctx.link().send_message(Msg::ShowDialog(format!(
                                                "Encryption failed: {}",
                                                e
                                            )));
                                        }
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
                    _ => {}
                }
                true
            }
            Msg::CopyPublicKey => {
                console::log!("📨 CopyPublicKey message received");
                if let Some(ref my_keys) = self.state.my_keys {
                    let public_key = my_keys.public_key.clone();
                    let js_code = format!(
                        "navigator.clipboard.writeText('{}').then(() => {{
                            console.log('✅ Public key copied to clipboard');
                            window.dispatchCustomEvent('show_dialog', 'Public key copied to clipboard!');
                        }}).catch((err) => {{
                            console.error('❌ Failed to copy public key:', err);
                            window.dispatchCustomEvent('show_dialog', 'Failed to copy to clipboard');
                        }});",
                        public_key.replace("'", "\\'")
                    );
                    let _ = js_sys::eval(&js_code);
                }
                true
            }
            Msg::CopyEncryptedMessage => {
                console::log!("📨 CopyEncryptedMessage message received");
                if let Some(ref encrypted_data) = self.state.encrypted_qr_data {
                    let js_code = format!(
                        "navigator.clipboard.writeText('{}').then(() => {{
                            console.log('✅ Encrypted message copied to clipboard');
                            window.dispatchCustomEvent('show_dialog', 'Encrypted message copied to clipboard!');
                        }}).catch((err) => {{
                            console.error('❌ Failed to copy encrypted message:', err);
                            window.dispatchCustomEvent('show_dialog', 'Failed to copy to clipboard');
                        }});",
                        encrypted_data.replace("'", "\\'")
                    );
                    let _ = js_sys::eval(&js_code);
                } else {
                    console::error!("❌ No encrypted message to copy");
                    dispatch_custom_event("show_dialog", "No encrypted message available to copy!");
                }
                true
            }
            Msg::DeleteContact(name) => {
                console::log!("📨 DeleteContact message received");
                console::log!(&format!("👤 Contact deleted: {}", name));
                self.delete_contact(name);
                true
            }
            Msg::HideDeleteConfirm => {
                console::log!("📨 HideDeleteConfirm message received");
                self.state.delete_confirm_visible = false;
                self.state.delete_target = None;
                true
            }
            Msg::ConfirmDelete => {
                console::log!("📨 ConfirmDelete message received");
                if let Some(ref name) = self.state.delete_target {
                    self.delete_contact(name.clone());
                }
                self.state.delete_confirm_visible = false;
                self.state.delete_target = None;
                true
            }
            Msg::SetDialogVisible(visible) => {
                console::log!("📨 SetDialogVisible message received");
                self.state.message_dialog_visible = visible;
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
                } else if let Some(ref _keys) = self.state.my_keys {
                    { self.render_main_view(ctx) }
                } else {
                    { self.render_loading_view() }
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
    fn initialize_app(&mut self, ctx: &Context<Self>) {
        console::log!("🔧 Application initialization started");
        let link = ctx.link().clone();

        ctx.link().send_message(Msg::UpdateLoadingProgress(
            "Checking saved keys...".to_string(),
            Some(10),
        ));

        spawn_local(async move {
            console::log!("💾 localStorage check started");
            delay_with_message(300).await;

            link.send_message(Msg::UpdateLoadingProgress(
                "Searching for keys...".to_string(),
                Some(20),
            ));
            delay_with_message(500).await;

            if let Some(keys) = load_my_keys().await {
                console::log!("✅ Existing keys found");

                link.send_message(Msg::UpdateLoadingProgress(
                    "Keys loaded successfully".to_string(),
                    Some(90),
                ));
                delay_with_message(400).await;

                let contacts = load_contacts().await;

                link.send_message(Msg::UpdateLoadingProgress(
                    "Application ready".to_string(),
                    Some(100),
                ));
                delay_with_message(500).await;

                link.send_message(Msg::SetLoading(false));
                link.send_message(Msg::KeysLoaded(keys, contacts));
            } else {
                console::log!("⚪ No existing keys found");
                link.send_message(Msg::UpdateLoadingProgress(
                    "Generating new keys...".to_string(),
                    Some(30),
                ));
                delay_with_message(600).await;
                link.send_message(Msg::GenerateKeys);
            }
        });
    }

    fn generate_new_keys(&mut self, ctx: &Context<Self>) {
        console::log!("🔧 New key generation started");
        let link = ctx.link().clone();

        ctx.link().send_message(Msg::UpdateLoadingProgress(
            "Starting key generation...".to_string(),
            Some(40),
        ));

        spawn_local(async move {
            delay_with_message(400).await;

            link.send_message(Msg::UpdateLoadingProgress(
                "Generating RSA key pair...".to_string(),
                Some(50),
            ));
            delay_with_message(300).await;

            match generate_key_pair_with_progress(link.clone()).await {
                Ok((public_key, private_key)) => {
                    console::log!("✅ RSA key generation completed");

                    link.send_message(Msg::UpdateLoadingProgress(
                        "Saving keys...".to_string(),
                        Some(85),
                    ));
                    delay_with_message(600).await;

                    save_my_keys(&private_key, &public_key).await;
                    console::log!("✅ Keys saved successfully");

                    let keys = KeyPair {
                        public_key: public_key.clone(),
                        private_key,
                    };

                    let contacts = HashMap::new();
                    link.send_message(Msg::UpdateLoadingProgress(
                        "Application ready".to_string(),
                        Some(100),
                    ));
                    delay_with_message(800).await;

                    link.send_message(Msg::SetLoading(false));
                    link.send_message(Msg::KeysLoaded(keys, contacts));

                    link.send_message(Msg::DrawQrCode(public_key));
                }
                Err(e) => {
                    console::error!(&format!("❌ Key generation error: {:?}", e));
                    link.send_message(Msg::UpdateLoadingProgress(
                        "Key generation failed".to_string(),
                        Some(0),
                    ));
                    delay_with_message(2000).await;
                    link.send_message(Msg::SetLoading(false));
                }
            }
        });
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

    fn decrypt_and_show_message(&mut self, encrypted_message: String) {
        if let Some(ref keys) = self.state.my_keys {
            let private_key = keys.private_key.clone();
            spawn_local(async move {
                match decrypt_message(&private_key, &encrypted_message).await {
                    Ok(decrypted) => {
                        dispatch_custom_event("show_dialog", &decrypted);
                    }
                    Err(e) => {
                        console::error!(&format!("Decryption failed: {:?}", e));
                        dispatch_custom_event("show_dialog", "Decryption failed");
                    }
                }
            });
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
                        <p>{"For security reasons, a 2048-bit RSA key is generated."}</p>
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
            </div>
        }
    }

    fn render_qr_reader(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::HideQrReader);

        html! {
            <div class="qr-reader-overlay">
                <div class="qr-reader">
                    <h3>{"Read QR"}</h3>
                    <p style="margin: 10px 0; color: #27ae60; font-size: 14px;">
                        {"📷 The camera is automatically activated..."}
                    </p>
                    <video id="qr-video" autoplay=true></video>
                    <div style="margin: 10px 0; text-align: center;">
                        <p style="font-size: 14px; color: #7f8c8d; margin-bottom: 10px;">
                            {"Please point the QR code to the camera"}
                        </p>
                    </div>

                    <div class="manual-input-section">
                        <h4>{"Manual Input"}</h4>
                        <p style="font-size: 13px; color: #7f8c8d; text-align: center; margin-bottom: 15px;">
                            {"Paste public key or encrypted message below"}
                        </p>

                        <div class="input-group">
                            <label>{"Public Key or Encrypted Message:"}</label>
                            <div class="textarea-container">
                                <textarea id="manual-input"
                                         placeholder="Paste public key (-----BEGIN PUBLIC KEY----- or Base64) or encrypted message here..."
                                         style="min-height: 100px;">
                                </textarea>
                                <button onclick={ctx.link().callback(|_| {
                                    if let Some(_window) = window() {
                                        if let Some(document) = _window.document() {
                                            if let Some(textarea) = document.get_element_by_id("manual-input") {
                                                if let Ok(textarea_element) = textarea.dyn_into::<HtmlTextAreaElement>() {
                                                    let input_data = textarea_element.value();
                                                    if !input_data.trim().is_empty() {
                                                        if let Some(qr_function) = js_sys::Reflect::get(&_window, &"processQrData".into()).ok() {
                                                            if qr_function.is_function() {
                                                                let _ = js_sys::Function::from(qr_function).call1(&_window, &JsValue::from_str(&input_data));
                                                            }
                                                        }
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

// 暗号化関連の関数

// 進行状況付き鍵生成関数
async fn generate_key_pair_with_progress(
    link: yew::html::Scope<App>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    console::log!("🔧 RSA key generation process started");

    link.send_message(Msg::UpdateLoadingProgress(
        "Generating random numbers...".to_string(),
        Some(60),
    ));
    delay_with_message(800).await;

    link.send_message(Msg::UpdateLoadingProgress(
        "Creating RSA key pair...".to_string(),
        Some(70),
    ));
    delay_with_message(1200).await;

    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits)?;
    let public_key = RsaPublicKey::from(&private_key);

    link.send_message(Msg::UpdateLoadingProgress(
        "Encoding keys...".to_string(),
        Some(80),
    ));
    delay_with_message(600).await;

    let private_pem = private_key.to_pkcs1_der()?;
    let public_pem = public_key.to_pkcs1_der()?;

    let private_key_str = BASE64.encode(private_pem.as_bytes());
    let public_key_str = BASE64.encode(public_pem.as_bytes());

    console::log!("✅ RSA key pair successfully generated");
    delay_with_message(500).await;

    Ok((public_key_str, private_key_str))
}

async fn delay_with_message(duration_ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = window() {
            let _ =
                window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration_ms);
        }
    });

    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

async fn decrypt_message(
    private_key: &str,
    encrypted_message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let private_key_bytes = BASE64.decode(private_key)?;
    let private_key = RsaPrivateKey::from_pkcs1_der(&private_key_bytes)?;

    let encrypted_bytes = BASE64.decode(encrypted_message)?;
    let decrypted = private_key.decrypt(Pkcs1v15Encrypt, &encrypted_bytes)?;

    Ok(String::from_utf8(decrypted)?)
}

// QRコード生成とCanvas描画
fn draw_qr_code_to_canvas(data: &str) {
    console::log!("🎨 QR code drawing started");
    console::log!(&format!("📊 Data length: {}", data.len()));
    console::log!(&format!("🔍 Data preview: {}", &data[..data.len().min(50)]));

    if let Some(window) = window() {
        let document = window.document().unwrap();

        let canvas = document
            .get_element_by_id("qr-canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        let viewport_width = window.inner_width().unwrap().as_f64().unwrap_or(800.0);
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

        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

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
}

// 暗号化されたメッセージのQRコード描画
fn draw_encrypted_qr_code_to_canvas(data: &str) {
    console::log!("🎨 Encrypted QR code drawing started");
    console::log!(&format!("📊 Encrypted message data length: {}", data.len()));
    console::log!(&format!(
        "🔑 Encrypted message preview: {}...",
        &data[..data.len().min(50)]
    ));

    match QrCode::new(data) {
        Ok(qr_code) => {
            console::log!("✅ Encrypted QR code generated successfully");
            let modules = qr_code.width();
            console::log!(&format!(
                "📐 Encrypted QR code size: {}x{}",
                modules, modules
            ));

            // Canvasを取得
            if let Some(window) = window() {
                console::log!("🌐 Window obtained successfully");
                if let Some(document) = window.document() {
                    console::log!("📄 Document obtained successfully");
                    if let Some(canvas_element) = document.get_element_by_id("encrypted-qr-canvas")
                    {
                        console::log!("🎯 Canvas element found");
                        match canvas_element.dyn_into::<HtmlCanvasElement>() {
                            Ok(canvas) => {
                                console::log!("🖼️ Canvas type conversion successful");

                                // 画面幅に応じてcanvasサイズを調整
                                let viewport_width =
                                    window.inner_width().unwrap().as_f64().unwrap_or(800.0);
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

                                match canvas.get_context("2d") {
                                    Ok(Some(context)) => {
                                        console::log!("🎨 2D context obtained successfully");
                                        match context.dyn_into::<CanvasRenderingContext2d>() {
                                            Ok(context) => {
                                                console::log!(
                                                    "✅ Encrypted CanvasRenderingContext2d obtained successfully"
                                                );

                                                // マージンを設定（角丸で欠けるのを防ぐ）
                                                let margin = 15.0; // 15px のマージン（暗号化QRは少し大きめ）
                                                let available_size =
                                                    canvas_size as f64 - (margin * 2.0);
                                                let cell_size = available_size / modules as f64;
                                                console::log!(&format!("📏 Encrypted margin: {}px, available size: {}px, cell size: {}", margin, available_size, cell_size));

                                                // 背景を白に
                                                context.set_fill_style_str("white");
                                                context.fill_rect(
                                                    0.0,
                                                    0.0,
                                                    canvas_size as f64,
                                                    canvas_size as f64,
                                                );
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
                                                console::log!(&format!("⚫ Encrypted QR code drawing completed - dark modules: {}", dark_modules));
                                                console::log!("🎉 Encrypted QR code drawing completed successfully!");
                                            }
                                            Err(e) => console::error!(&format!(
                                                "❌ CanvasRenderingContext2d type conversion error: {:?}",
                                                e
                                            )),
                                        }
                                    }
                                    Ok(None) => console::error!("❌ 2D context is null"),
                                    Err(e) => console::error!(&format!(
                                        "❌ 2D context obtaining error: {:?}",
                                        e
                                    )),
                                }
                            }
                            Err(e) => console::error!(&format!(
                                "❌ Canvas type conversion error: {:?}",
                                e
                            )),
                        }
                    } else {
                        console::error!("❌ Canvas element not found! ID: encrypted-qr-canvas");
                        // DOM内の全canvas要素を検索
                        let canvas_list = document.query_selector_all("canvas").unwrap();
                        console::log!(&format!(
                            "🔍 Number of canvas elements in DOM: {}",
                            canvas_list.length()
                        ));
                    }
                } else {
                    console::error!("❌ Document obtaining failed");
                }
            } else {
                console::error!("❌ Window obtaining failed");
            }
        }
        Err(e) => {
            console::error!(&format!("❌ Encrypted QR code generation error: {:?}", e));
        }
    }
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

fn get_local_storage() -> Option<Storage> {
    window()?.local_storage().ok()?
}

// 同期版の暗号化関数
fn encrypt_message_sync(
    public_key: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
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

#[wasm_bindgen(start)]
pub fn main() {
    yew::Renderer::<App>::new().render();
}
