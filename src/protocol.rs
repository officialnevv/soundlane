//! Copy/paste signalling and data-channel messages.
//!
//! The invitation deliberately contains only portable signalling metadata. SDP, ICE
//! candidates, and credentials stay inside the transport adapter; this makes the
//! human-visible code versionable without exposing transport internals to the TUI.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invite {
    pub version: u8,
    pub room_id: String,
    pub host_name: String,
    pub source_label: String,
    pub offer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Answer {
    pub version: u8,
    pub room_id: String,
    pub guest_name: String,
    pub answer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    Hello { name: String },
    Chat { text: String },
    Permission { stream: bool, media: bool },
    PlayPause,
    Handoff { successor_id: String },
    Status { buffered_ms: f32, rtt_ms: Option<f32> },
}

#[derive(Debug, Error)]
pub enum CodeError {
    #[error("code is missing the expected {0} prefix")]
    Prefix(&'static str),
    #[error("code is damaged or incomplete")]
    Encoding(#[from] base64::DecodeError),
    #[error("code is not valid Soundlane data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this code uses unsupported protocol version {0}")]
    Version(u8),
}

impl Invite {
    pub fn encode(&self) -> String {
        format!("sl-invite-{}", URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("serializable invite")))
    }

    pub fn decode(value: &str) -> Result<Self, CodeError> {
        decode("sl-invite-", value)
    }
}

impl Answer {
    pub fn encode(&self) -> String {
        format!("sl-answer-{}", URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("serializable answer")))
    }

    pub fn decode(value: &str) -> Result<Self, CodeError> {
        decode("sl-answer-", value)
    }
}

fn decode<T: for<'de> Deserialize<'de>>(prefix: &'static str, value: &str) -> Result<T, CodeError> {
    let value = value.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
    let payload = value.strip_prefix(prefix).ok_or(CodeError::Prefix(prefix))?;
    let parsed: T = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    // Version is checked by parsing the shared wire header separately. This keeps
    // messages extensible while still rejecting incompatible future protocol data.
    let header: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    let version = header.get("version").and_then(serde_json::Value::as_u64).unwrap_or(0) as u8;
    if version != PROTOCOL_VERSION { return Err(CodeError::Version(version)); }
    Ok(parsed)
}

pub fn sanitize_chat(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_control()).collect::<String>().trim().chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invite_round_trips_and_ignores_paste_whitespace() {
        let invite = Invite { version: 1, room_id: "r1".into(), host_name: "Ava".into(), source_label: "Music".into(), offer: "sdp".into() };
        let code = invite.encode();
        assert_eq!(Invite::decode(&format!("  {code}\n")).unwrap(), invite);
    }
    #[test]
    fn chat_is_bounded_and_printable() { assert_eq!(sanitize_chat(" hi\nthere "), "hithere"); }
}
