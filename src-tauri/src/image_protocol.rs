/// Tauri encodes the full custom-protocol path. Decode once before splitting
/// the route from its URL or absolute path, on both Windows and macOS.
pub fn parse_image_uri(uri: &str) -> Result<(String, String), &'static str> {
    let rest = uri
        .strip_prefix("http://jmimg.localhost/")
        .or_else(|| uri.strip_prefix("https://jmimg.localhost/"))
        .or_else(|| uri.strip_prefix("jmimg://localhost/"))
        .or_else(|| uri.strip_prefix("jmimg://"))
        .ok_or("bad scheme")?;
    let decoded = urlencoding::decode(rest).map_err(|_| "bad encoding")?;
    let (kind, target) = decoded.split_once('/').ok_or("bad request")?;
    let kind = kind.to_ascii_lowercase();
    if target.is_empty() || !["remote", "local", "photo"].contains(&kind.as_str()) {
        return Err("bad route");
    }
    Ok((kind, target.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_and_macos_preserve_image_urls_and_absolute_paths() {
        for (kind, target) in [
            (
                "remote",
                "https://images.example.test/封面%20一.png?token=a%2Fb",
            ),
            ("local", "/Users/reader/漫画/100%20/第 1 页.png"),
            ("local", "C:\\Books\\漫画\\100%20\\第 1 页.png"),
            ("photo", "123456/第 1 页.webp"),
        ] {
            let encoded = urlencoding::encode(&format!("{kind}/{target}")).into_owned();
            for prefix in [
                "http://jmimg.localhost/",
                "https://jmimg.localhost/",
                "jmimg://localhost/",
            ] {
                assert_eq!(
                    parse_image_uri(&format!("{prefix}{encoded}")).unwrap(),
                    (kind.to_string(), target.to_string())
                );
            }
        }
    }

    #[test]
    fn legacy_routes_preserve_unix_roots_and_encoded_characters() {
        let target = "/Volumes/Books/漫画/封面%20一.png";
        for prefix in ["jmimg://", "http://jmimg.localhost/"] {
            assert_eq!(
                parse_image_uri(&format!("{prefix}local/{}", urlencoding::encode(target))).unwrap(),
                ("local".to_string(), target.to_string())
            );
        }
    }

    #[test]
    fn invalid_origins_routes_and_encodings_are_rejected() {
        for uri in [
            "https://other.test/local/a.png",
            "jmimg://localhost/local",
            "jmimg://localhost/local/",
            "jmimg://localhost/execute%2Ffile",
            "jmimg://localhost/photo%2F%FF",
        ] {
            assert!(parse_image_uri(uri).is_err(), "{uri}");
        }
    }
}
