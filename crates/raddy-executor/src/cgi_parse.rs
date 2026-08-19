//! CGI/1.1 stdout → [`ResponseHead`] plus body.

use raddy_abi::ResponseHead;

use crate::ExecError;

/// Parsed CGI response. Headers keep repeated pairs in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CgiParsed {
    pub head: ResponseHead,
    pub body: Vec<u8>,
}

/// Split CGI stdout on the first header/body separator (`\r\n\r\n` or `\n\n`).
pub fn parse_cgi_response(raw: &[u8]) -> Result<CgiParsed, ExecError> {
    let Some((header_end, body_start)) = find_separator(raw) else {
        return Err(ExecError::Protocol(
            "missing CGI header/body separator".into(),
        ));
    };
    let text = String::from_utf8_lossy(&raw[..header_end]);
    let mut status = 200_u16;
    let mut headers = Vec::new();
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ExecError::Protocol(format!(
                "malformed CGI header line: {line}"
            )));
        };
        let name = name.trim();
        let value = value.trim();
        if name.eq_ignore_ascii_case("status") {
            status = parse_cgi_status(value)?;
            continue;
        }
        headers.push((name.to_ascii_lowercase(), value.to_string()));
    }
    Ok(CgiParsed {
        head: ResponseHead::new(status, headers),
        body: raw[body_start..].to_vec(),
    })
}

fn find_separator(raw: &[u8]) -> Option<(usize, usize)> {
    if let Some(i) = find_subslice(raw, b"\r\n\r\n") {
        return Some((i, i + 4));
    }
    find_subslice(raw, b"\n\n").map(|i| (i, i + 2))
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn parse_cgi_status(value: &str) -> Result<u16, ExecError> {
    let code = value
        .split_whitespace()
        .next()
        .ok_or_else(|| ExecError::Protocol("empty CGI Status".into()))?;
    code.parse::<u16>()
        .map_err(|_| ExecError::Protocol(format!("invalid CGI Status: {value}")))
}
