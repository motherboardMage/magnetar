# Magnetar Architecture

## Overview
TUI BitTorrent client in Rust. Zero-copy Nom parsing of torrent files and tracker responses. Async networking via Tokio. Supports BitTorrent v1, v2, and hybrid torrents.

## Modules

### `torrent_file` (done)
- **Purpose:** Zero-copy Nom parsing of `.torrent` files (Bencode format). Handles v1 single-file, v1 multi-file, v2 (file tree), and hybrid layouts.
- **Sub-modules:**
  - `parser` — All Bencode and torrent-specific parsers
  - `skip_values` — Skips unknown Bencode values without allocating
- **Public API:**
  - `pub fn parse_torrent_file(input: &'a [u8]) -> IResult<&'a [u8], Torrent<'a>>`
- **`pub(crate)` primitives** (shared with tracker module):
  - `fail()`, `res_in_some()`, `parse_bencode_int()`, `parse_bencode_string()`
  - `skip_bencode_value()`
- **Borrows from:** The caller's `&[u8]` buffer. All parsed data (`Torrent<'a>`, `Info<'a>`, etc.) borrows directly from this buffer — zero heap copies for string fields.

### `tracker` (in progress)
- **Purpose:** HTTP tracker communication (announce protocol, BEP 3).
- **Sub-modules:**
  - `http` — URL building, response parsing, async announce function
- **Done:**
  - `fn urlencode_bytes(bytes: &[u8]) -> String` — RFC 3986 percent-encoding for raw binary
  - `fn build_tracker_url(base_url: &str, req: &TrackerRequest) -> String` — Constructs announce query string
  - `fn parse_compact_peers(input: &[u8]) -> IResult<...>` — Parses 6-byte compact peer format (4-byte IPv4 + 2-byte big-endian port)
  - `fn parse_tracker_response(input: &[u8]) -> IResult<&[u8], TrackerResponse>` — Bencode dict parser for tracker replies
- **In progress:**
  - `pub async fn announce(base_url: &str, req: &TrackerRequest<'_>) -> Result<TrackerResponse, Box<dyn std::error::Error>>`
- **Depends on:** `torrent_file::parser` for Bencode primitives, `reqwest` for HTTP, `tokio` for async runtime.
- **Note:** `TrackerResponse` and `AnnounceResponse` are lifetime-free (owned types), making them trivially `Send` across Tokio tasks.

### `peer` (not started)
- **Purpose:** Peer wire protocol over raw TCP (BEP 3). Handshake, message framing (choke/unchoke/interested/bitfield/request/piece/cancel), per-peer connection state machine.
- **Will use:** `tokio::net::TcpStream`, binary message parsing.

### piece manager (not started)
- **Purpose:** Piece selection (rarest-first), 16 KB block request pipelining, SHA-1 piece verification against `pieces` blob from torrent file.

### disk (not started)
- **Purpose:** Write verified pieces to files on disk. Handle piece-to-file spanning for multi-file torrents. Crash-safe persistent resume state.

### TUI (not started)
- **Purpose:** Terminal user interface for download progress, peer stats, etc.

## How the modules talk to each other

```
┌─────────────────┐
│  .torrent file  │ (bytes on disk)
└────────┬────────┘
         │ std::fs::read()
         ▼
┌─────────────────┐
│  torrent_file   │ parse_torrent_file(&bytes) -> Torrent<'a>
│  (parser)       │ Extracts: announce URL, info_hash (via raw_info), piece hashes, file layout
└────────┬────────┘
         │ announce URL + info_hash
         ▼
┌─────────────────┐
│  tracker::http  │ announce(url, req).await -> TrackerResponse
│  (async)        │ Returns: Vec<SocketAddr> of peers, re-announce interval
└────────┬────────┘
         │ peer addresses
         ▼
┌─────────────────┐
│  peer (future)  │ TCP connections, handshakes, block exchange
└────────┬────────┘
         │ verified pieces
         ▼
┌─────────────────┐
│  disk (future)  │ Write pieces to files
└─────────────────┘
```

**Concurrency model (planned, not implemented):**
- ? — Channels vs shared state for peer ↔ piece manager communication not yet decided.
- ? — Whether piece manager is a dedicated Tokio task or integrated into a central coordinator not yet decided.

## Core types shared across modules

| Type | Module | Crosses into | Lifetime? |
|------|--------|-------------|-----------|
| `Torrent<'a>` | `torrent_file` | `main`, `tracker` (for announce URL + raw_info) | Yes — borrows from file buffer |
| `Info<'a>` | `torrent_file` | `main`, future piece manager | Yes |
| `FileLayout<'a>` | `torrent_file` | `main`, future disk module | Yes |
| `TrackerRequest<'a>` | `tracker` | `tracker::http` | Yes — borrows info_hash and peer_id |
| `TrackerResponse` | `tracker` | `main`, future peer module | **No** — fully owned, trivially `Send` |
| `AnnounceResponse` | `tracker` | `main`, future peer module | **No** |
| `SocketAddr` | `std::net` | Everywhere (tracker → peer connections) | **No** |

## Open questions
- How will `Torrent<'a>` (which borrows from the file buffer) be stored long-term alongside its owning buffer without self-referential struct issues? Deferred to download manager implementation.
- One unified error enum across the crate, or one per module?
- Channel-based vs shared-state architecture for peer ↔ piece manager communication?
- UDP tracker support (BEP 15) — planned but not designed yet.
- DHT support (BEP 5) — not planned yet.
- `piece_layers` cross-referencing with file tree `pieces_root` — deferred to a separate validation pass, not done at parse time.
