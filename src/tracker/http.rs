use nom::AsChar;
use reqwest;
use std::fmt::Write;

use crate::tracker::Event;
use crate::tracker::TrackerRequest;

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
}
