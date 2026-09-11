use crate::{Error, Result};
use futures_util::StreamExt;
use reqwest::{header, Client};
use url::Url;

pub struct Response {
    pub status: u16,
    pub location: Option<String>,
    pub text: String,
    pub skipped: bool,
}

/// Redirects are deliberately handled by the queue so every hop obeys scope/robots.
pub async fn fetch(client: &Client, url: &Url, limit: usize, document: bool) -> Result<Response> {
    let response = client.get(url.clone()).send().await?;
    let status = response.status().as_u16();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let skipped = document && !matches!(mime.as_str(), "text/html" | "application/xhtml+xml");
    if skipped || !(200..300).contains(&status) {
        return Ok(Response {
            status,
            location,
            text: String::new(),
            skipped,
        });
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(Error::Invalid(format!("response exceeds {limit} bytes")));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::Invalid(format!("response exceeds {limit} bytes")));
        }
        bytes.extend_from_slice(&chunk);
    }
    // Also accept .xml.gz files without a Content-Encoding header.
    if !document && bytes.starts_with(&[0x1f, 0x8b]) {
        use std::io::Read;
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(bytes.as_slice())
            .take(limit as u64 + 1)
            .read_to_end(&mut decoded)?;
        if decoded.len() > limit {
            return Err(Error::Invalid(
                "decompressed sitemap exceeds body limit".into(),
            ));
        }
        bytes = decoded;
    }
    let charset = content_type.split(';').skip(1).find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        key.eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches('"'))
    });
    let encoding = charset
        .and_then(|c| encoding_rs::Encoding::for_label(c.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    let text = encoding.decode(&bytes).0.into_owned();
    Ok(Response {
        status,
        location,
        text,
        skipped,
    })
}
