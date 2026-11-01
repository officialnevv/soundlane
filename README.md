# Soundlane

Soundlane is a Rust terminal client for sharing a listening session. It is a rewrite of the supplied Python prototype with a Ratatui room interface and a deliberately separated protocol, synchronisation, audio, and transport design.

## Current foundation

- `soundlane host` opens a host room with chat, peer status, invitations, and room commands.
- `soundlane join` accepts a copy/pasted `sl-invite-…` code in the TUI.
- Versioned, URL-safe invite and answer codes; bounded/sanitised chat messages.
- NTP-style clock-sync and a jitter-buffer core with unit tests, ready to be fed by an Opus/WebRTC audio adapter.
- The TUI supports `/help`, `/invite`, `/status`, `/chat`, `/trim`, `/vol`, `/stream`, `/playpause`, and `/quit`.

## Architecture

The original script owned terminal input, WebRTC callbacks, audio capture, and playout in one module. Soundlane separates them so each platform integration remains replaceable:

```
Ratatui app ── control protocol ── WebRTC data channels
                    │                       │
              invite / answer          Opus packets
                    │                       │
               clock sync ────────── jitter buffer ── audio output
```

`protocol.rs` owns human-copyable signaling data and control messages. `sync.rs` is intentionally platform-neutral, so audio adapters can supply decoded 48 kHz stereo frames without changing UI or room semantics.

## Run

Install stable Rust, then:

```sh
cargo run -- host --name Ava --source Spotify --stun stun:stun.l.google.com:19302
cargo run -- join --name Ben
```

The transport offer is intentionally marked `pending-transport-offer` until the WebRTC adapter is connected. This prevents the new UI scaffold from advertising a false, connectable room. The next implementation slice is the `webrtc` data-channel adapter, followed by platform capture (WASAPI process loopback, PipeWire/PulseAudio, and a macOS virtual device), then Opus encode/decode.

## Controls

Press `Enter` to send, `Esc` to clear input, or `Ctrl-C`/`/quit` to leave. Host invites are displayed with `/invite`.
