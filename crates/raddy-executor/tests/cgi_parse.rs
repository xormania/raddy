use raddy_executor::parse_cgi_response;

#[test]
fn status_line_and_body() {
    let raw = b"Status: 201 Created\r\nContent-Type: text/plain\r\n\r\nhello";
    let got = parse_cgi_response(raw).expect("parse");
    assert_eq!(got.head.status, 201);
    assert_eq!(got.body, b"hello");
    assert!(
        got.head
            .headers
            .iter()
            .any(|(n, v)| n == "content-type" && v == "text/plain")
    );
    assert!(
        !got.head.headers.iter().any(|(n, _)| n == "status"),
        "CGI Status is not an HTTP header"
    );
}

#[test]
fn lf_only_separator_defaults_to_200() {
    let raw = b"Content-Type: text/plain\n\nHi";
    let got = parse_cgi_response(raw).expect("parse");
    assert_eq!(got.head.status, 200);
    assert_eq!(got.body, b"Hi");
}

#[test]
fn repeated_set_cookie_preserves_order() {
    let raw = b"Set-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\n";
    let got = parse_cgi_response(raw).expect("parse");
    let cookies: Vec<_> = got
        .head
        .headers
        .iter()
        .filter(|(n, _)| n == "set-cookie")
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(cookies, ["a=1", "b=2"]);
}

#[test]
fn missing_header_body_separator_is_protocol() {
    let err = parse_cgi_response(b"Status: 200 OK\r\nno blank line").expect_err("need separator");
    let msg = err.to_string();
    assert!(
        msg.contains("separator") || msg.contains("protocol"),
        "expected a protocol error, got {msg}"
    );
}
