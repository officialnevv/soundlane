use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::protocol::{sanitize_chat, Invite, PROTOCOL_VERSION};
use crate::sync::{ClockSync, JitterBuffer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomMode { Host, Guest }

#[derive(Clone, Debug)]
pub struct Peer { pub id: String, pub name: String, pub rtt_ms: Option<f32>, pub connected: bool }

#[derive(Clone, Debug)]
pub struct ChatLine { pub from: String, pub text: String, pub own: bool }

#[derive(Debug)]
pub struct App {
    pub mode: RoomMode,
    pub name: String,
    pub source: String,
    pub bitrate: u32,
    pub stun: Option<String>,
    pub turn: Option<String>,
    pub room_id: String,
    pub input: String,
    pub notice: String,
    pub chat: Vec<ChatLine>,
    pub peers: Vec<Peer>,
    pub invite: Option<String>,
    pub connected: bool,
    pub should_quit: bool,
    pub clock: ClockSync,
    pub playout: JitterBuffer<Vec<f32>>,
    started: Instant,
}

impl App {
    pub fn host(name: String, source: String, bitrate: u32, stun: Option<String>, turn: Option<String>) -> Self {
        let room_id = format!("{:08x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos());
        let invite = Invite {
            version: PROTOCOL_VERSION, room_id: room_id.clone(), host_name: name.clone(), source_label: source.clone(),
            // This is replaced by a WebRTC offer before sending. Keeping the placeholder
            // explicit prevents a UI-only invite from accidentally being treated as connectable.
            offer: "pending-transport-offer".into(),
        }.encode();
        Self::new(RoomMode::Host, name, source, bitrate, stun, turn, room_id, Some(invite), "Ready — press i to show your invite, or type /help.")
    }

    pub fn guest(name: String, stun: Option<String>, turn: Option<String>) -> Self {
        Self::new(RoomMode::Guest, name, "waiting for host".into(), 128, stun, turn, String::new(), None, "Paste an sl-invite code, then press Enter.")
    }

    fn new(mode: RoomMode, name: String, source: String, bitrate: u32, stun: Option<String>, turn: Option<String>, room_id: String, invite: Option<String>, notice: &str) -> Self {
        Self { mode, name, source, bitrate, stun, turn, room_id, input: String::new(), notice: notice.into(), chat: vec![], peers: vec![], invite, connected: false, should_quit: false, clock: ClockSync::default(), playout: JitterBuffer::default(), started: Instant::now() }
    }

    pub fn push_input(&mut self, ch: char) { self.input.push(ch); }
    pub fn backspace(&mut self) { self.input.pop(); }
    pub fn clear_input(&mut self) { self.input.clear(); }
    pub fn quit(&mut self) { self.should_quit = true; }

    pub fn submit_input(&mut self) {
        let line = std::mem::take(&mut self.input);
        let trimmed = line.trim();
        if trimmed.is_empty() { return; }
        if !trimmed.starts_with('/') && self.mode == RoomMode::Guest && !self.connected && trimmed.starts_with("sl-invite-") {
            self.join_from_invite(trimmed);
        } else if let Some(command) = trimmed.strip_prefix('/') { self.command(command); }
        else { self.say(trimmed); }
    }

    pub fn tick(&mut self) {
        // The transport feeds real sync samples and packets into these primitives.
        // This timer only keeps the status area alive before the first peer connects.
        if self.connected && self.clock.rtt_seconds.is_none() { self.clock.add_sample(0.0, 0.025, 0.05); }
    }

    fn join_from_invite(&mut self, input: &str) {
        match Invite::decode(input) {
            Ok(invite) if invite.offer == "pending-transport-offer" => {
                self.room_id = invite.room_id;
                self.source = invite.source_label;
                self.notice = format!("Invite from {} loaded. WebRTC negotiation will produce your answer here.", invite.host_name);
            }
            Ok(invite) => {
                self.room_id = invite.room_id;
                self.source = invite.source_label;
                self.connected = true;
                self.peers.push(Peer { id: "host".into(), name: invite.host_name, rtt_ms: None, connected: true });
                self.notice = "Connected. Type a message or /help for commands.".into();
            }
            Err(error) => self.notice = format!("Could not read invite: {error}"),
        }
    }

    fn command(&mut self, command: &str) {
        let mut parts = command.split_whitespace();
        let verb = parts.next().unwrap_or_default().to_ascii_lowercase();
        match verb.as_str() {
            "quit" | "exit" | "q" => self.quit(),
            "help" => self.notice = "/help  /invite  /status  /chat <message>  /trim <ms>  /vol <0-100>  /quit".into(),
            "invite" | "i" if self.mode == RoomMode::Host => self.notice = self.invite.clone().unwrap_or_else(|| "Offer is not ready yet.".into()),
            "status" => self.notice = self.status(),
            "chat" => self.say(parts.collect::<Vec<_>>().join(" ").trim()),
            "trim" => self.notice = format!("Playout trim set to {} ms.", parts.next().unwrap_or("0")),
            "vol" => self.notice = format!("Output volume set to {}%.", parts.next().unwrap_or("100")),
            "playpause" => self.notice = "Play/pause request queued for the host media adapter.".into(),
            "stream" => self.notice = "A permitted guest stream is mixed by the audio adapter.".into(),
            _ => self.notice = "Unknown command. Type /help.".into(),
        }
    }

    fn say(&mut self, value: &str) {
        let text = sanitize_chat(value);
        if text.is_empty() { return; }
        self.chat.push(ChatLine { from: self.name.clone(), text, own: true });
        if self.chat.len() > 200 { self.chat.remove(0); }
    }

    pub fn status(&self) -> String {
        let rtt = self.clock.rtt_seconds.map(|v| format!("{:.0}ms", v * 1_000.0)).unwrap_or_else(|| "—".into());
        format!("{} · {} · {} peer(s) · RTT {} · {} gaps / {} resyncs", self.source, if self.connected { "connected" } else { "waiting" }, self.peers.len(), rtt, self.playout.gaps, self.playout.resyncs)
    }

    pub fn elapsed(&self) -> Duration { self.started.elapsed() }
}
