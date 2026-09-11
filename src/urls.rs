use url::Url;

/// Remove fragments; preserve potentially meaningful path/query/scheme differences.
pub fn normalize(mut url: Url) -> Option<Url> {
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return None;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    url.set_fragment(None);
    // RFC 3986 unreserved escapes are cosmetic; reserved escapes stay encoded.
    let mut normalized = String::with_capacity(url.as_str().len());
    let mut chars = url.as_str().chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let a = chars.next();
            let b = chars.next();
            if let (Some(a), Some(b)) = (a, b) {
                if let (Some(hi), Some(lo)) = (a.to_digit(16), b.to_digit(16)) {
                    let byte = (hi * 16 + lo) as u8;
                    if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                        normalized.push(byte as char);
                    } else {
                        normalized.push('%');
                        normalized.push(a.to_ascii_uppercase());
                        normalized.push(b.to_ascii_uppercase());
                    }
                } else {
                    normalized.push('%');
                    normalized.push(a);
                    normalized.push(b);
                }
            } else {
                normalized.push('%');
                if let Some(a) = a {
                    normalized.push(a);
                }
            }
        } else {
            normalized.push(c);
        }
    }
    Url::parse(&normalized).ok()
}

pub fn resolve(base: &Url, href: &str) -> Option<Url> {
    let href = href.trim();
    if href.is_empty() || href.starts_with('#') {
        return None;
    }
    normalize(base.join(href).ok()?)
}

pub fn internal(root: &Url, url: &Url, subdomains: bool) -> bool {
    match (root.host_str(), url.host_str()) {
        (Some(a), Some(b)) => a == b || (subdomains && b.ends_with(&format!(".{a}"))),
        _ => false,
    }
}

pub fn html_candidate(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    ![
        "jpg", "jpeg", "png", "gif", "webp", "svg", "ico", "avif", "css", "js", "mjs", "pdf",
        "zip", "gz", "tar", "rar", "7z", "mp3", "mp4", "wav", "webm", "ogg", "woff", "woff2",
        "ttf", "eot", "exe", "dmg", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "xml", "json",
        "rss", "atom", "wasm",
    ]
    .iter()
    .any(|ext| path.ends_with(&format!(".{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolution_and_identity() {
        let base = Url::parse("https://EXAMPLE.com:443/a/b").unwrap();
        assert_eq!(
            resolve(&base, "../c?q=1#x").unwrap().as_str(),
            "https://example.com/c?q=1"
        );
        for value in ["#x", "mailto:a@b.com", "tel:1", "javascript:void(0)", ""] {
            assert!(resolve(&base, value).is_none());
        }
        assert_ne!(resolve(&base, "/c"), resolve(&base, "/c/"));
        assert_eq!(resolve(&base, "/%7euser"), resolve(&base, "/~user"));
        assert_ne!(resolve(&base, "/a%2fb"), resolve(&base, "/a/b"));
        assert_ne!(resolve(&base, "/?a=1&b=2"), resolve(&base, "/?b=2&a=1"));
        assert!(!internal(
            &base,
            &Url::parse("https://notexample.com").unwrap(),
            true
        ));
        assert!(internal(
            &base,
            &Url::parse("http://sub.example.com").unwrap(),
            true
        ));
    }
}
