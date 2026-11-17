use crate::torrent_file::parser::{fail, parse_bencode_int, parse_bencode_string, res_in_some};
use crate::torrent_file::skip_values::skip_bencode_value;

use nom::IResult;
use nom::bytes::complete::{tag, take};
use nom::combinator::peek;
use nom::error::ErrorKind;
use nom::{AsChar, Parser};

use reqwest;
use std::any::Any;
use std::fmt::Write;
use std::net::{Ipv4Addr, SocketAddr};

use crate::tracker::{AnnounceResponse, TrackerRequest};
use crate::tracker::{Event, TrackerResponse};

fn urlencode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 3);

    let allowed_symbols = [b'.', b'_', b'-', b'~'];
    for b in bytes {
        if b.is_ascii_alphanumeric() || allowed_symbols.contains(b) {
            encoded.push(b.as_char());
        } else {
            let ch = format!("%{:02X}", b);
            encoded.push_str(ch.as_str());
        }
    }
    encoded
}

fn build_tracker_url(base_url: &str, req: &TrackerRequest) -> String {
    let mut tracker_url = base_url.to_string();

    if base_url.contains('?') {
        if !base_url.ends_with('&') && !base_url.ends_with('?') {
            write!(tracker_url, "&").unwrap();
        }
    } else {
        write!(tracker_url, "?").unwrap();
    }

    write!(
        tracker_url,
        "info_hash={ih}&peer_id={pi}&port={p}&uploaded={ul}&downloaded={dl}&left={l}",
        ih = urlencode_bytes(req.info_hash),
        pi = urlencode_bytes(req.peer_id),
        p = req.port,
        ul = req.uploaded,
        dl = req.downloaded,
        l = req.left,
    )
    .unwrap();

    if let Some(ev) = req.event {
        match ev {
            Event::Started => write!(tracker_url, "&event=started").unwrap(),
            Event::Completed => write!(tracker_url, "&event=completed").unwrap(),
            Event::Stopped => write!(tracker_url, "&event=stopped").unwrap(),
        }
    }

    if let Some(ip) = req.ip {
        write!(tracker_url, "&ip={ip}").unwrap();
    }

    write!(tracker_url, "&compact={}", req.compact).unwrap();

    tracker_url
}

fn parse_compact_peers(input: &[u8]) -> IResult<&[u8], Vec<std::net::SocketAddr>> {
    let (input, data) = parse_bencode_string(input)?;

    if data.len() % 6 != 0 {
        return Err(fail(input, nom::error::ErrorKind::LengthValue));
    }

    let mut peers = Vec::with_capacity(data.len() / 6);
    for chunk in data.chunks_exact(6) {
        let ipv4 = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        peers.push(SocketAddr::from((ipv4, port)));
    }

    Ok((input, peers))
}

fn parse_tracker_response(input: &[u8]) -> IResult<&[u8], TrackerResponse> {
    let (mut input, _) = tag("d").parse(input)?;
    let mut failure_reason = None;
    let mut interval = None;
    let mut peers = None;

    loop {
        let (_, mut next) = peek(take(1usize)).parse(input)?;

        if next == b"e" {
            break;
        }

        (input, next) = parse_bencode_string(input)?;

        match next {
            b"failure reason" => {
                (input, failure_reason) = res_in_some(parse_bencode_string(input))?
            }
            b"interval" => (input, interval) = res_in_some(parse_bencode_int(input))?,
            b"peers" => (input, peers) = res_in_some(parse_compact_peers(input))?,
            _ => (input, _) = skip_bencode_value(input)?,
        }
    }

    (input, _) = tag("e").parse(input)?;

    if let Some(f) = failure_reason {
        Ok((
            input,
            TrackerResponse::Failure(String::from_utf8_lossy(f).into_owned()),
        ))
    } else {
        let interval = match interval {
            Some(i) if i > 0 => i as u32,
            _ => return Err(fail(input, ErrorKind::Fail)),
        };
        let peers = peers.ok_or(fail(input, ErrorKind::Fail))?;

        Ok((
            input,
            TrackerResponse::Success(AnnounceResponse { interval, peers }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode_standard() {
        let bytes = [b'A', b'g', b'4', b'.', b'~', 0x1A];
        let expected = String::from("Ag4.~%1A");

        assert_eq!(urlencode_bytes(&bytes), expected);
    }

    #[test]
    fn test_urlencode_single_digit_hex() {
        let bytes = [0x00, 0x05, 0x0F];
        assert_eq!(urlencode_bytes(&bytes), "%00%05%0F");
    }

    #[test]
    fn test_build_url_standard() {
        let base_url = "http://tracker.example.com/announce";

        let req = TrackerRequest {
            info_hash: &[b'a'; 20],
            peer_id: &[b'c'; 20],
            ip: Some("192.168.1.123".parse().unwrap()),
            port: 6881u16,
            uploaded: 1_000_000_000u64,
            downloaded: 2_000_000_000u64,
            left: 3_000_000_000u64,
            event: Some(Event::Started),
            compact: 1u8,
        };

        assert_eq!(
            build_tracker_url(base_url, &req),
            format!(
                "http://tracker.example.com/announce?info_hash={ih}&peer_id={pi}&port=6881&uploaded=1000000000&downloaded=2000000000&left=3000000000&event=started&ip=192.168.1.123&compact=1",
                ih = "a".repeat(20),
                pi = "c".repeat(20)
            )
        );
    }

    #[test]
    fn test_build_url_existing_parameter() {
        let base_url = "http://tracker.example.com/announce?some=bullshit";

        let req = TrackerRequest {
            info_hash: &[b'a'; 20],
            peer_id: &[b'c'; 20],
            ip: Some("192.168.1.123".parse().unwrap()),
            port: 6881u16,
            uploaded: 1_000_000_000u64,
            downloaded: 2_000_000_000u64,
            left: 3_000_000_000u64,
            event: Some(Event::Started),
            compact: 1u8,
        };

        assert_eq!(
            build_tracker_url(base_url, &req),
            format!(
                "http://tracker.example.com/announce?some=bullshit&info_hash={ih}&peer_id={pi}&port=6881&uploaded=1000000000&downloaded=2000000000&left=3000000000&event=started&ip=192.168.1.123&compact=1",
                ih = "a".repeat(20),
                pi = "c".repeat(20)
            )
        );
    }

    #[test]
    fn test_build_url_trailing_delimiter() {
        let base_url = "http://tracker.example.com/announce?";
        let base_url_2 = "http://tracker.example.com/announce?some=bullshit&";

        let req = TrackerRequest {
            info_hash: &[b'a'; 20],
            peer_id: &[b'c'; 20],
            ip: Some("192.168.1.123".parse().unwrap()),
            port: 6881u16,
            uploaded: 1_000_000_000u64,
            downloaded: 2_000_000_000u64,
            left: 3_000_000_000u64,
            event: Some(Event::Started),
            compact: 1u8,
        };

        assert_eq!(
            build_tracker_url(base_url, &req),
            format!(
                "http://tracker.example.com/announce?info_hash={ih}&peer_id={pi}&port=6881&uploaded=1000000000&downloaded=2000000000&left=3000000000&event=started&ip=192.168.1.123&compact=1",
                ih = "a".repeat(20),
                pi = "c".repeat(20)
            )
        );

        assert_eq!(
            build_tracker_url(base_url_2, &req),
            format!(
                "http://tracker.example.com/announce?some=bullshit&info_hash={ih}&peer_id={pi}&port=6881&uploaded=1000000000&downloaded=2000000000&left=3000000000&event=started&ip=192.168.1.123&compact=1",
                ih = "a".repeat(20),
                pi = "c".repeat(20)
            )
        );
    }

    // --- Tracker response parser tests ---

    #[test]
    fn test_parse_tracker_response_standard() {
        let expected = TrackerResponse::Success(AnnounceResponse {
            interval: 1800u32,
            peers: vec![
                SocketAddr::from((Ipv4Addr::new(127, 0, 0, 1), 6881)),
                SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 80)),
            ],
        });

        assert_eq!(
            parse_tracker_response(
                b"d8:intervali1800e5:peers12:\x7f\x00\x00\x01\x1a\xe1\x01\x02\x03\x04\x00\x50e"
            ),
            Ok((b"".as_slice(), expected))
        )
    }

    #[test]
    fn test_parse_tracker_response_extra_keys() {
        let expected = TrackerResponse::Success(AnnounceResponse {
            interval: 1800u32,
            peers: vec![
                SocketAddr::from((Ipv4Addr::new(127, 0, 0, 1), 6881)),
                SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 80)),
            ],
        });

        assert_eq!(
            parse_tracker_response(
                b"d8:completei10e8:intervali1800e10:incompletei10e5:peers12:\x7f\x00\x00\x01\x1a\xe1\x01\x02\x03\x04\x00\x50e"
            ),
            Ok((b"".as_slice(), expected))
        )
    }

    #[test]
    fn test_parse_tracker_response_failure() {
        let expected = TrackerResponse::Failure("torrent not registered".into());

        assert_eq!(
            parse_tracker_response(b"d14:failure reason22:torrent not registerede"),
            Ok((b"".as_slice(), expected))
        )
    }
}
