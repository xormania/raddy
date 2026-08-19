use std::net::SocketAddr;

use http::{HeaderMap, Method, Uri};
use raddy_abi::{BodyMeta, Envelope, RequestId};

/// Map an HTTP request line and headers to a v1 [`Envelope`].
///
/// Header names are lowercased. Repeated headers stay as repeated pairs, order
/// preserved. `body.len` is `None` when Content-Length is absent.
pub fn envelope_from_http(
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
    deadline_ms: u64,
) -> Envelope {
    let target = uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let authority = uri
        .authority()
        .map(|a| a.as_str().to_string())
        .or_else(|| {
            headers
                .get(http::header::HOST)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let mut mapped = Vec::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        mapped.push((
            name.as_str().to_ascii_lowercase(),
            String::from_utf8_lossy(value.as_bytes()).into_owned(),
        ));
    }
    let len = headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    Envelope {
        v: raddy_abi::ABI_VERSION,
        request_id: RequestId::new(),
        method: method.as_str().to_string(),
        target,
        scheme: "http".into(),
        authority,
        headers: mapped,
        remote_addr,
        body: BodyMeta { len },
        deadline_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::envelope_from_http;
    use http::{HeaderMap, HeaderValue, Method, Uri};

    fn peer() -> std::net::SocketAddr {
        "127.0.0.1:54321".parse().expect("peer")
    }

    #[test]
    fn repeated_headers_preserve_order_and_pairs() {
        let mut headers = HeaderMap::new();
        headers.append("Set-Cookie", HeaderValue::from_static("a=1"));
        headers.append("Set-Cookie", HeaderValue::from_static("b=2"));
        headers.append("Accept", HeaderValue::from_static("*/*"));
        let env = envelope_from_http(
            &Method::GET,
            &"/".parse::<Uri>().unwrap(),
            &headers,
            peer(),
            30_000,
        );
        assert_eq!(
            env.headers,
            vec![
                ("set-cookie".into(), "a=1".into()),
                ("set-cookie".into(), "b=2".into()),
                ("accept".into(), "*/*".into()),
            ]
        );
    }

    #[test]
    fn absent_content_length_is_unknown_body_len() {
        let env = envelope_from_http(
            &Method::POST,
            &"/upload".parse::<Uri>().unwrap(),
            &HeaderMap::new(),
            peer(),
            1_000,
        );
        assert_eq!(env.body.len, None);
        assert_eq!(env.target, "/upload");
        assert_eq!(env.method, "POST");
    }

    #[test]
    fn authority_prefers_uri_then_host_header() {
        let uri: Uri = "http://uri.example:8080/p?q=1".parse().unwrap();
        let env = envelope_from_http(&Method::GET, &uri, &HeaderMap::new(), peer(), 30_000);
        assert_eq!(env.authority, "uri.example:8080");
        assert_eq!(env.target, "/p?q=1");

        let mut host_only = HeaderMap::new();
        host_only.insert(
            http::header::HOST,
            HeaderValue::from_static("host.example:9"),
        );
        let env = envelope_from_http(
            &Method::GET,
            &"/only".parse::<Uri>().unwrap(),
            &host_only,
            peer(),
            30_000,
        );
        assert_eq!(env.authority, "host.example:9");

        let env = envelope_from_http(
            &Method::GET,
            &"/bare".parse::<Uri>().unwrap(),
            &HeaderMap::new(),
            peer(),
            30_000,
        );
        assert_eq!(env.authority, "");
    }
}
