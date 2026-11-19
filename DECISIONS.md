# Decisions

Format: date, decision, reason, alternatives rejected.

---

## [2026-08] No generic Bencode AST
- **Decision:** Parse Bencode directly into domain-specific Rust structs (`Torrent`, `Info`, `FileLayout`, etc.) instead of building a generic `BencodeValue` enum tree first.
- **Why:** A generic AST forces heap allocations everywhere (`Vec`, `HashMap`) and loses compile-time knowledge of field types. Direct parsing enables zero-copy borrowing from the input buffer.
- **Rejected:** `enum BencodeValue { Dict(...), List(...), Int(i64), Bytes(Vec<u8>) }` — too many allocations, no type safety on fields.

## [2026-08] Zero-copy `&'a [u8]` for all string fields
- **Decision:** All parsed string/byte fields borrow directly from the input buffer as `&'a [u8]` slices. No `String` or `Vec<u8>` copies.
- **Why:** Torrent files can be large (especially the `pieces` blob). Copying megabytes of hash data into owned `Vec`s is wasteful when the input buffer already lives in memory.
- **Rejected:** `String` / `Vec<u8>` owned fields.

## [2026-08] `pieces` stored as flat `&'a [u8]`, not `Vec<[u8; 20]>`
- **Decision:** Keep the SHA-1 pieces blob as a single contiguous `&'a [u8]` slice. Validate `pieces.len() % 20 == 0` at parse time. Add an accessor method later for individual piece hashes.
- **Why:** Splitting into a `Vec` of 20-byte arrays forces heap allocation and copies. The flat slice is zero-copy and can be chunked on demand with `.chunks_exact(20)`.
- **Rejected:** `Vec<[u8; 20]>` — unnecessary allocation.

## [2026-08] `usize` for `length` and `piece_length`
- **Decision:** Use `usize` for file sizes and piece lengths, with a compile-time assertion that the platform is 64-bit.
- **Why:** Files can exceed 4 GB (u32 limit). `usize` on 64-bit platforms is 8 bytes and matches Rust's indexing/slicing expectations.
- **Rejected:** `u64` — requires casts every time you index into a slice.

## [2026-08] Unified Option-based error pattern for dict parsers
- **Decision:** Accumulate all dictionary fields as `Option<T>` during the parsing loop, then validate with `.ok_or(fail(...))` after the loop ends. Missing fields and invalid values both result in `None`.
- **Why:** Keeps every dict parser following one consistent pattern. Splitting into within-loop vs post-loop error checking adds complexity without benefit.
- **Rejected:** Separate error handling for missing vs invalid values inside the loop.

## [2026-08] `BTreeMap` for file tree directories
- **Decision:** Use `BTreeMap<&'a [u8], FileTreeNode<'a>>` for v2 file tree directory entries.
- **Why:** Maintains sorted key order (matching Bencode's canonical form) and is simpler than a custom tree structure.
- **Rejected:** Custom tree struct — debated but `BTreeMap` kept for simplicity.

## [2026-08] Version detection via tuple matching
- **Decision:** At the end of `parse_info_dict`, match on `(file_tree, files, length, pieces)` as a tuple of Options to determine the `FileLayout` variant (V1Single, V1Multi, V2, Hybrid).
- **Why:** Clean, exhaustive, and the compiler verifies all combinations are handled. Invalid combinations (e.g. `file_tree` present with `length` but no `pieces`) produce errors.
- **Rejected:** Sequential if-else chains — less readable, easier to miss cases.

## [2026-08] `pieces_root` is `Option<&'a [u8]>`
- **Decision:** `pieces_root` in `FileTreeNode::File` is optional because zero-length files have no pieces and therefore no Merkle root.
- **Why:** The BitTorrent v2 spec (BEP 52) explicitly states zero-length files omit `pieces root`.

## [2026-08] `piece_layers` validation deferred
- **Decision:** Parse `piece_layers` as a simple `BTreeMap<&[u8], &[u8]>` with basic validation (keys = 32 bytes, values divisible by 32). Cross-referencing with file tree `pieces_root` entries is deferred to a separate validation pass.
- **Why:** Mixing parsing with cross-module validation makes the parser harder to test and reason about.

## [2026-08] Capture `raw_info` via slice length arithmetic
- **Decision:** Capture the raw info dictionary bytes for SHA-1 hashing using `&info_start[..info_start.len() - info_end.len()]` where `info_start` and `info_end` are the input slices before and after parsing the info dict.
- **Why:** Zero-copy, no pointer arithmetic needed. Works because both slices point into the same underlying buffer.

## [2026-10] Tokio over Rayon for async runtime
- **Decision:** Use Tokio as the async runtime for all networking and concurrency.
- **Why:** BitTorrent is I/O-bound (waiting on network sockets from dozens of peers), not CPU-bound. Tokio's event-driven model (epoll/kqueue) efficiently multiplexes hundreds of connections on a few OS threads. `tokio::task::spawn_blocking` available for occasional CPU-heavy work (e.g. SHA-1 verification).
- **Rejected:** Rayon — designed for CPU-parallel computation (thread pool), not I/O concurrency. Threads would block waiting on network packets.

## [2026-10] reqwest for HTTP tracker, raw TCP for peer wire
- **Decision:** Use `reqwest` for HTTP tracker announces. Write the peer wire protocol directly on `tokio::net::TcpStream`.
- **Why:** HTTP tracker is a simple GET request — not architecturally interesting. The real systems learning is in the peer wire protocol (custom binary framing, state machines), which no library handles for us. `reqwest` pulls in Tokio anyway, so it's not wasted dependency weight.
- **Rejected:** Writing a raw HTTP client — educational value is low compared to the peer wire protocol.

## [2026-11] `pub(crate)` for Bencode primitives
- **Decision:** Bencode primitive parsers (`parse_bencode_int`, `parse_bencode_string`, `fail`, `res_in_some`) are `pub(crate)`. Domain-specific parsers (`parse_file_entry`, `parse_info_dict`, etc.) stay private.
- **Why:** Bencode is the universal wire format of BitTorrent (torrent files, tracker responses, DHT, peer extensions). Primitives need to be shared across modules. Domain parsers are internal to their module.
- **Rejected:** Making everything `pub` — leaks internal implementation into the public API.

## [2026-11] Lifetime-free `TrackerResponse` and `AnnounceResponse`
- **Decision:** `TrackerResponse` and `AnnounceResponse` are fully owned types (no lifetime parameters). Peers stored as `Vec<SocketAddr>`, failure reason as owned `String`.
- **Why:** Lifetime-free types are trivially `Send` and can be moved across Tokio task boundaries (`tokio::spawn`) without any borrow checker friction. The tracker response is small and short-lived — owning the data is cheap.
- **Rejected:** `&'a [u8]` peer_id in a `Peer` struct — compact mode doesn't even include peer_id; it's learned later during the TCP handshake.

## [2026-11] `announce` returns `Box<dyn std::error::Error>`
- **Decision:** `async fn announce(...)  -> Result<TrackerResponse, Box<dyn std::error::Error>>` uses a trait object error type.
- **Why:** The function combines errors from two different sources (reqwest network errors and Nom parse errors). `Box<dyn Error>` avoids inventing a custom error enum before the error landscape is fully known.
- **Rejected (for now):** Custom `TrackerError` enum — premature; error design should stabilize after more modules exist. Can be introduced later.

## [2026-11] Compact peers only (`compact=1`)
- **Decision:** Always request and parse compact peer format (6 bytes per peer: 4-byte IPv4 + 2-byte big-endian port).
- **Why:** Compact format is vastly more bandwidth-efficient and is supported by virtually all modern trackers. The non-compact dictionary format is legacy bloat.
- **Rejected:** Supporting non-compact peer dictionaries — unnecessary complexity for no practical benefit.
