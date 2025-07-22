use crate::common::RtcMessage;
use gloo::console;
use js_sys::{Array, Object, Reflect};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Event, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent, RtcDataChannelInit,
    RtcDataChannelState, RtcIceConnectionState, RtcPeerConnection, RtcPeerConnectionIceEvent,
    RtcSdpType, RtcSessionDescriptionInit,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionDescription {
    pub sdp_type: String,
    pub sdp: String,
}

fn create_ice_servers() -> Array {
    let servers = Array::new();

    let stun_config = Object::new();
    let _ = Reflect::set(
        &stun_config,
        &"urls".into(),
        &"stun:stun.l.google.com:19302".into(),
    );
    servers.push(&stun_config);

    let stun_config2 = Object::new();
    let _ = Reflect::set(
        &stun_config2,
        &"urls".into(),
        &"stun:stun1.l.google.com:19302".into(),
    );
    servers.push(&stun_config2);

    let stun_config6 = Object::new();
    let _ = Reflect::set(
        &stun_config6,
        &"urls".into(),
        &"stun:stun.services.mozilla.com".into(),
    );
    servers.push(&stun_config6);

    servers
}

#[derive(Clone)]
pub struct Connection {
    pc: RtcPeerConnection,
    channel: Arc<Mutex<Option<RtcDataChannel>>>,
    is_initiator: bool,
    current_signal: Arc<Mutex<Option<String>>>,
    on_connection_established: Arc<Mutex<Option<Box<dyn Fn() + 'static>>>>,
    on_data_channel_open: Arc<Mutex<Option<Box<dyn Fn() + 'static>>>>,
}

impl Connection {
    pub fn new() -> Self {
        let config = RtcConfiguration::new();
        let ice_servers = create_ice_servers();
        config.set_ice_servers(&ice_servers);

        let pc = RtcPeerConnection::new_with_configuration(&config)
            .expect("Failed to create RtcPeerConnection");

        let connection = Self {
            pc,
            channel: Arc::new(Mutex::new(None)),
            is_initiator: false,
            current_signal: Arc::new(Mutex::new(None)),
            on_connection_established: Arc::new(Mutex::new(None)),
            on_data_channel_open: Arc::new(Mutex::new(None)),
        };

        connection.setup_ice_monitoring();

        connection
    }

    fn setup_ice_monitoring(&self) {
        let pc_for_state = self.pc.clone();
        let on_connection_callback = Arc::clone(&self.on_connection_established);
        let ice_connection_state_closure = Closure::wrap(Box::new(move |_event: Event| {
            let state = pc_for_state.ice_connection_state();
            console::log!("🔗 ICE connection state changed:", format!("{:?}", state));

            match state {
                RtcIceConnectionState::Connected | RtcIceConnectionState::Completed => {
                    console::log!("✅ WebRTC connection established!");
                    if let Ok(callback_guard) = on_connection_callback.lock() {
                        if let Some(ref callback) = *callback_guard {
                            callback();
                        }
                    }
                }
                RtcIceConnectionState::Failed => {
                    console::log!("❌ WebRTC connection failed; This may be due to a firewall or network issue.");
                }
                RtcIceConnectionState::Disconnected => {
                    console::log!("⚠️ WebRTC connection disconnected!");
                }
                RtcIceConnectionState::Closed => {
                    console::log!("🔒 WebRTC connection closed!");
                }
                _ => {
                    // 他の状態（New, Checking）は特にアクションなし
                }
            }
        }) as Box<dyn FnMut(_)>);

        self.pc.set_oniceconnectionstatechange(Some(
            ice_connection_state_closure.as_ref().unchecked_ref(),
        ));
        ice_connection_state_closure.forget();

        // ICE gathering状態の監視を設定
        let ice_gathering_state_closure = Closure::wrap(Box::new(move |event: Event| {
            console::log!("🔍 ICE gathering state changed", event.target());
        }) as Box<dyn FnMut(_)>);

        self.pc.set_onicegatheringstatechange(Some(
            ice_gathering_state_closure.as_ref().unchecked_ref(),
        ));
        ice_gathering_state_closure.forget();
    }

    pub fn set_connection_established_handler(
        &self,
        handler: impl Fn() + 'static,
    ) -> Result<(), JsValue> {
        if let Ok(mut callback_guard) = self.on_connection_established.lock() {
            *callback_guard = Some(Box::new(handler));
        }
        Ok(())
    }

    pub fn set_data_channel_open_handler(
        &self,
        handler: impl Fn() + 'static,
    ) -> Result<(), JsValue> {
        if let Ok(mut callback_guard) = self.on_data_channel_open.lock() {
            *callback_guard = Some(Box::new(handler));
        }
        Ok(())
    }

    pub async fn start_connection(
        &mut self,
        callback: impl Fn(String) + 'static,
    ) -> Result<(), JsValue> {
        self.is_initiator = true;

        let channel_init = RtcDataChannelInit::new();
        channel_init.set_ordered(true);
        let channel = self
            .pc
            .create_data_channel_with_data_channel_dict("data", &channel_init);

        // データチャネルオープンイベントハンドラを設定
        let on_data_channel_callback = Arc::clone(&self.on_data_channel_open);
        let open_closure = Closure::wrap(Box::new(move |_event: Event| {
            console::log!("🔓 Data channel opened! Ready for messaging");
            if let Ok(callback_guard) = on_data_channel_callback.lock() {
                if let Some(ref callback) = *callback_guard {
                    callback();
                }
            }
        }) as Box<dyn FnMut(_)>);

        channel.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
        open_closure.forget();

        if let Ok(mut channel_guard) = self.channel.lock() {
            *channel_guard = Some(channel);
        }

        // ICE candidate イベントハンドラを設定
        let pc_clone = self.pc.clone();
        let callback_closure = Closure::wrap(Box::new(move |event: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = event.candidate() {
                // ICE candidateが発見された時のログ
                console::log!("🧊 ICE candidate found:", candidate.candidate());
            } else {
                // ICE gathering 完了時にコールバックを呼び出し
                console::log!("✅ ICE gathering completed");
                if let Some(desc) = pc_clone.local_description() {
                    let session_desc = SessionDescription {
                        sdp_type: "offer".to_string(),
                        sdp: desc.sdp(),
                    };
                    if let Ok(offer_json) = serde_json::to_string(&session_desc) {
                        callback(offer_json);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        self.pc
            .set_onicecandidate(Some(callback_closure.as_ref().unchecked_ref()));
        callback_closure.forget();

        // Offer を作成
        let offer = JsFuture::from(self.pc.create_offer()).await?;
        let offer_sdp = Reflect::get(&offer, &"sdp".into())?.as_string().unwrap();

        let offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.set_sdp(&offer_sdp);

        JsFuture::from(self.pc.set_local_description(&offer_obj)).await?;

        Ok(())
    }

    pub async fn recv_offer(
        &mut self,
        offer: String,
        callback: impl Fn(String) + 'static,
    ) -> Result<(), JsValue> {
        let channel_arc = self.channel.clone();
        let on_data_channel_callback = Arc::clone(&self.on_data_channel_open);

        // データチャネルイベントハンドラを設定
        let datachannel_closure = Closure::wrap(Box::new(move |event: RtcDataChannelEvent| {
            let channel = event.channel();

            // 受信したデータチャネルにオープンハンドラを設定
            let callback_for_open = Arc::clone(&on_data_channel_callback);
            let open_closure = Closure::wrap(Box::new(move |_event: Event| {
                console::log!("🔓 Data channel opened (Answer side)! Ready for messaging");
                if let Ok(callback_guard) = callback_for_open.lock() {
                    if let Some(ref callback) = *callback_guard {
                        callback();
                    }
                }
            }) as Box<dyn FnMut(_)>);

            channel.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
            open_closure.forget();

            if let Ok(mut channel_guard) = channel_arc.lock() {
                *channel_guard = Some(channel);
            }
        }) as Box<dyn FnMut(_)>);

        self.pc
            .set_ondatachannel(Some(datachannel_closure.as_ref().unchecked_ref()));
        datachannel_closure.forget();

        let session_desc: SessionDescription = serde_json::from_str(&offer)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse offer: {}", e)))?;

        // Remote description を設定
        let offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.set_sdp(&session_desc.sdp);

        JsFuture::from(self.pc.set_remote_description(&offer_obj)).await?;

        // ICE candidate イベントハンドラを設定
        let pc_clone = self.pc.clone();
        let callback_closure = Closure::wrap(Box::new(move |event: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = event.candidate() {
                // ICE candidateが発見された時のログ
                console::log!("🧊 ICE candidate found (Answer):", candidate.candidate());
            } else {
                // ICE gathering 完了時にコールバックを呼び出し
                console::log!("✅ ICE gathering completed (Answer)");
                if let Some(desc) = pc_clone.local_description() {
                    let session_desc = SessionDescription {
                        sdp_type: "answer".to_string(),
                        sdp: desc.sdp(),
                    };
                    if let Ok(answer_json) = serde_json::to_string(&session_desc) {
                        callback(answer_json);
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        self.pc
            .set_onicecandidate(Some(callback_closure.as_ref().unchecked_ref()));
        callback_closure.forget();

        // Answer を作成
        let answer = JsFuture::from(self.pc.create_answer()).await?;
        let answer_sdp = Reflect::get(&answer, &"sdp".into())?.as_string().unwrap();

        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(&answer_sdp);

        JsFuture::from(self.pc.set_local_description(&answer_obj)).await?;

        Ok(())
    }

    pub async fn recv_answer(&mut self, answer: String) -> Result<(), JsValue> {
        let session_desc: SessionDescription = serde_json::from_str(&answer)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse answer: {}", e)))?;

        let answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.set_sdp(&session_desc.sdp);

        JsFuture::from(self.pc.set_remote_description(&answer_obj)).await?;
        Ok(())
    }

    pub fn process_signal(&mut self, signal_data: &str) -> Result<(), JsValue> {
        if let Ok(mut signal_guard) = self.current_signal.lock() {
            *signal_guard = Some(signal_data.to_string());
        }
        Ok(())
    }

    pub fn send_message(&mut self, message: &RtcMessage) -> Result<(), JsValue> {
        if let Ok(channel_guard) = self.channel.lock() {
            if let Some(channel) = &*channel_guard {
                // データチャネルの状態をチェック
                let ready_state = channel.ready_state();
                if ready_state != RtcDataChannelState::Open {
                    return Err(JsValue::from_str(&format!(
                        "Data channel is not ready (state: {:?}). Cannot send message.",
                        ready_state
                    )));
                }

                let message_str = serde_json::to_string(message).map_err(|e| {
                    JsValue::from_str(&format!("Failed to serialize message: {}", e))
                })?;
                channel.send_with_str(&message_str)?;
            } else {
                return Err(JsValue::from_str("No data channel available"));
            }
        }
        Ok(())
    }

    pub fn send_data(&self, data: &str) -> Result<(), JsValue> {
        if let Ok(channel_guard) = self.channel.lock() {
            if let Some(channel) = &*channel_guard {
                channel.send_with_str(data)?;
            }
        }
        Ok(())
    }

    pub fn set_data_handler(&self, handler: impl Fn(String) + 'static) -> Result<(), JsValue> {
        if let Ok(channel_guard) = self.channel.lock() {
            if let Some(channel) = &*channel_guard {
                let message_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
                    if let Some(data) = event.data().as_string() {
                        handler(data);
                    }
                }) as Box<dyn FnMut(_)>);

                channel.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
                message_closure.forget();
            }
        }
        Ok(())
    }

    pub fn wait_for_open(&self, callback: impl Fn() + 'static) -> Result<(), JsValue> {
        if let Ok(channel_guard) = self.channel.lock() {
            if let Some(channel) = &*channel_guard {
                // 既にオープン状態の場合は即座にコールバックを実行
                if channel.ready_state() == RtcDataChannelState::Open {
                    callback();
                    return Ok(());
                }

                // オープンイベントハンドラを設定
                let open_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    callback();
                }) as Box<dyn FnMut(_)>);

                channel.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
                open_closure.forget();
            }
        }
        Ok(())
    }

    pub fn close(&self) {
        if let Ok(channel_guard) = self.channel.lock() {
            if let Some(channel) = &*channel_guard {
                channel.close();
            }
        }
        self.pc.close();
    }
}
