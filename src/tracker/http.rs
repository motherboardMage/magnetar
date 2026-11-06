use nom::AsChar;
use reqwest;

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
