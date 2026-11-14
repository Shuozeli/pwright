//! Bounded HTTP/1 wire encoding and decoding for API request transports.
//!
//! This crate intentionally owns no socket, executor, TLS implementation,
//! proxy implementation, redirect policy or decompressor. A transport plugin
//! feeds received bytes into [`Http1ResponseDecoder`] and retains exact header
//! spelling, ordering and duplicates for Playwright-compatible `headersArray`.

use pwright_api_request::{ApiHeader, TransportRequest};

const MAX_CHUNK_SIZE_LINE_BYTES: usize = 1024;
const MIN_HEAD_EVENT_FRAGMENT_ALLOWANCE: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Http1ResponseHead {
    pub version: u8,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<ApiHeader>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Http1Response {
    pub version: u8,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<ApiHeader>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Http1ResponseEvent {
    Head(Http1ResponseHead),
    Complete(Http1Response),
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum Http1CodecError {
    #[error("invalid HTTP/1 request: {0}")]
    InvalidRequest(String),
    #[error("HTTP/1 response exceeded {0}")]
    Capacity(&'static str),
    #[error("malformed HTTP/1 response: {0}")]
    MalformedResponse(String),
    #[error("truncated HTTP/1 response while reading {0}")]
    Truncated(&'static str),
    #[error("HTTP/1 response contained bytes after the final response")]
    TrailingBytes,
    #[error("HTTP/1 response decoder is already complete")]
    AlreadyComplete,
}

/// Encode one request for a connection that owns exactly one exchange.
///
/// `Host` and `Connection: close` are added when absent. Existing header names,
/// values, order and duplicates are emitted unchanged.
///
/// # Errors
///
/// Returns [`Http1CodecError`] when the request is not representable as
/// HTTP/1 or its encoded header section exceeds the configured bound.
pub fn encode_request(request: &TransportRequest) -> Result<Vec<u8>, Http1CodecError> {
    if request.method.is_empty() || !request.method.bytes().all(is_token_byte) {
        return Err(Http1CodecError::InvalidRequest(
            "method must be a non-empty HTTP token".into(),
        ));
    }
    if !matches!(request.url.scheme(), "http" | "https") {
        return Err(Http1CodecError::InvalidRequest(
            "URL must use HTTP or HTTPS".into(),
        ));
    }

    let mut target = request.url.path().to_owned();
    if target.is_empty() {
        target.push('/');
    }
    if let Some(query) = request.url.query() {
        target.push('?');
        target.push_str(query);
    }
    if target.bytes().any(|byte| byte <= 0x20 || byte == 0x7f) {
        return Err(Http1CodecError::InvalidRequest(
            "request target contains a control or space".into(),
        ));
    }

    let mut head = Vec::new();
    append_request_head(
        &mut head,
        request.method.as_bytes(),
        request.max_request_header_bytes,
    )?;
    append_request_head(&mut head, b" ", request.max_request_header_bytes)?;
    append_request_head(
        &mut head,
        target.as_bytes(),
        request.max_request_header_bytes,
    )?;
    append_request_head(
        &mut head,
        b" HTTP/1.1\r\n",
        request.max_request_header_bytes,
    )?;

    for header in &request.headers {
        append_header(&mut head, header, request.max_request_header_bytes)?;
    }
    if !has_header(&request.headers, "host") {
        append_header(
            &mut head,
            &ApiHeader::new("Host", host_header_value(request)?),
            request.max_request_header_bytes,
        )?;
    }
    if !has_header(&request.headers, "connection") {
        append_header(
            &mut head,
            &ApiHeader::new("Connection", "close"),
            request.max_request_header_bytes,
        )?;
    }
    append_request_head(&mut head, b"\r\n", request.max_request_header_bytes)?;

    let body_len = request.body.as_ref().map_or(0, Vec::len);
    let output_len = head
        .len()
        .checked_add(body_len)
        .ok_or(Http1CodecError::Capacity("encoded request bytes"))?;
    let mut output = Vec::with_capacity(output_len);
    output.extend_from_slice(&head);
    if let Some(body) = &request.body {
        output.extend_from_slice(body);
    }
    Ok(output)
}

fn host_header_value(request: &TransportRequest) -> Result<String, Http1CodecError> {
    let host = match request.url.host() {
        Some(url::Host::Domain(value)) => value.to_owned(),
        Some(url::Host::Ipv4(value)) => value.to_string(),
        Some(url::Host::Ipv6(value)) => format!("[{value}]"),
        None => {
            return Err(Http1CodecError::InvalidRequest("URL has no host".into()));
        }
    };
    Ok(match request.url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

fn append_header(
    output: &mut Vec<u8>,
    header: &ApiHeader,
    limit: usize,
) -> Result<(), Http1CodecError> {
    if header.name.is_empty() || !header.name.bytes().all(is_token_byte) {
        return Err(Http1CodecError::InvalidRequest(
            "header name is not an HTTP token".into(),
        ));
    }
    if !header.value.chars().all(is_header_value_char) {
        return Err(Http1CodecError::InvalidRequest(
            "header value is not representable as HTTP/1 Latin-1".into(),
        ));
    }
    append_request_head(output, header.name.as_bytes(), limit)?;
    append_request_head(output, b": ", limit)?;
    for character in header.value.chars() {
        append_request_head(output, &[character as u8], limit)?;
    }
    append_request_head(output, b"\r\n", limit)
}

fn append_request_head(
    output: &mut Vec<u8>,
    value: &[u8],
    limit: usize,
) -> Result<(), Http1CodecError> {
    let next_len = output
        .len()
        .checked_add(value.len())
        .ok_or(Http1CodecError::Capacity("request header bytes"))?;
    if next_len > limit {
        return Err(Http1CodecError::Capacity("request header bytes"));
    }
    output.extend_from_slice(value);
    Ok(())
}

fn has_header(headers: &[ApiHeader], name: &str) -> bool {
    headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case(name))
}

#[derive(Debug)]
pub struct Http1ResponseDecoder {
    request_is_head: bool,
    max_body_bytes: usize,
    max_headers: usize,
    max_header_bytes: usize,
    max_status_text_bytes: usize,
    head_bytes_seen: usize,
    buffer: Vec<u8>,
    state: DecodeState,
}

#[derive(Debug)]
enum DecodeState {
    Head,
    Fixed {
        response: Http1Response,
        expected: usize,
    },
    Chunked {
        response: Http1Response,
        phase: ChunkPhase,
    },
    UntilEof(Http1Response),
    Complete,
    Poisoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChunkPhase {
    Size,
    Data(usize),
    DataCrlf,
    Trailers,
}

impl Http1ResponseDecoder {
    #[must_use]
    pub fn new(request: &TransportRequest) -> Self {
        Self {
            request_is_head: request.method.eq_ignore_ascii_case("HEAD"),
            max_body_bytes: request.max_response_body_bytes,
            max_headers: request.max_response_headers,
            max_header_bytes: request.max_response_header_bytes,
            max_status_text_bytes: request.max_response_status_text_bytes,
            head_bytes_seen: 0,
            buffer: Vec::new(),
            state: DecodeState::Head,
        }
    }

    /// Feed an arbitrary response fragment. Returns exactly one final response.
    ///
    /// # Errors
    ///
    /// Returns [`Http1CodecError`] for malformed, truncated, over-capacity or
    /// trailing response data, or when data is fed after completion.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Option<Http1Response>, Http1CodecError> {
        let mut event = self.feed_event(bytes)?;
        loop {
            match event {
                Some(Http1ResponseEvent::Head(_)) => event = self.feed_event(&[])?,
                Some(Http1ResponseEvent::Complete(response)) => return Ok(Some(response)),
                None => return Ok(None),
            }
        }
    }

    /// Feed a response fragment and stop at the final response head before
    /// consuming its body. Calling again resumes body decoding from retained
    /// bounded bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Http1CodecError`] for malformed, truncated, over-capacity or
    /// trailing response data, or when data is fed after completion.
    pub fn feed_event(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<Http1ResponseEvent>, Http1CodecError> {
        if matches!(self.state, DecodeState::Complete) {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(Http1CodecError::AlreadyComplete)
            };
        }
        self.append_input(bytes)?;
        self.process_event(false)
    }

    /// Signal connection EOF. Close-delimited bodies become complete here;
    /// fixed-length and chunked bodies must already be structurally complete.
    ///
    /// # Errors
    ///
    /// Returns [`Http1CodecError`] when the response is incomplete or a
    /// configured response boundary was exceeded.
    pub fn finish(&mut self) -> Result<Option<Http1Response>, Http1CodecError> {
        let mut event = self.finish_event()?;
        loop {
            match event {
                Some(Http1ResponseEvent::Head(_)) => event = self.finish_event()?,
                Some(Http1ResponseEvent::Complete(response)) => return Ok(Some(response)),
                None => return Ok(None),
            }
        }
    }

    /// Signal connection EOF while preserving the head-before-body event
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns [`Http1CodecError`] when the response is incomplete or a
    /// configured response boundary was exceeded.
    pub fn finish_event(&mut self) -> Result<Option<Http1ResponseEvent>, Http1CodecError> {
        if matches!(self.state, DecodeState::Complete) {
            return Ok(None);
        }
        self.process_event(true)
    }

    fn append_input(&mut self, bytes: &[u8]) -> Result<(), Http1CodecError> {
        let ordinary_wire_limit = self
            .max_body_bytes
            .checked_add(self.max_header_bytes.saturating_mul(2))
            .and_then(|value| value.checked_add(MAX_CHUNK_SIZE_LINE_BYTES))
            .unwrap_or(usize::MAX);
        let head_event_wire_limit = self
            .max_header_bytes
            .saturating_add(MIN_HEAD_EVENT_FRAGMENT_ALLOWANCE);
        let wire_limit = ordinary_wire_limit.max(head_event_wire_limit);
        let next_len = self
            .buffer
            .len()
            .checked_add(bytes.len())
            .ok_or(Http1CodecError::Capacity("buffered response bytes"))?;
        if next_len > wire_limit {
            return Err(Http1CodecError::Capacity("buffered response bytes"));
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    fn process_event(&mut self, eof: bool) -> Result<Option<Http1ResponseEvent>, Http1CodecError> {
        loop {
            let state = std::mem::replace(&mut self.state, DecodeState::Poisoned);
            match state {
                DecodeState::Head => match self.parse_head()? {
                    HeadProgress::Incomplete => {
                        self.state = DecodeState::Head;
                        return if eof {
                            Err(Http1CodecError::Truncated("response headers"))
                        } else {
                            Ok(None)
                        };
                    }
                    HeadProgress::Informational => {
                        self.state = DecodeState::Head;
                    }
                    HeadProgress::Final(response, body_mode) => {
                        let head = Http1ResponseHead {
                            version: response.version,
                            status: response.status,
                            status_text: response.status_text.clone(),
                            headers: response.headers.clone(),
                        };
                        match body_mode {
                            BodyMode::None => {
                                self.state = DecodeState::Fixed {
                                    response,
                                    expected: 0,
                                };
                            }
                            BodyMode::Fixed(expected) => {
                                self.state = DecodeState::Fixed { response, expected };
                            }
                            BodyMode::Chunked => {
                                self.state = DecodeState::Chunked {
                                    response,
                                    phase: ChunkPhase::Size,
                                };
                            }
                            BodyMode::UntilEof => {
                                self.state = DecodeState::UntilEof(response);
                            }
                        }
                        return Ok(Some(Http1ResponseEvent::Head(head)));
                    }
                },
                DecodeState::Fixed {
                    mut response,
                    expected,
                } => {
                    if expected > self.max_body_bytes {
                        return Err(Http1CodecError::Capacity("response body bytes"));
                    }
                    let remaining = expected.saturating_sub(response.body.len());
                    let take = remaining.min(self.buffer.len());
                    response.body.extend_from_slice(&self.buffer[..take]);
                    self.buffer.drain(..take);
                    if response.body.len() == expected {
                        return self.complete(response);
                    }
                    self.state = DecodeState::Fixed { response, expected };
                    return if eof {
                        Err(Http1CodecError::Truncated("fixed-length response body"))
                    } else {
                        Ok(None)
                    };
                }
                DecodeState::Chunked {
                    mut response,
                    mut phase,
                } => match self.process_chunked(&mut response, &mut phase)? {
                    ChunkProgress::Incomplete => {
                        self.state = DecodeState::Chunked { response, phase };
                        return if eof {
                            Err(Http1CodecError::Truncated("chunked response body"))
                        } else {
                            Ok(None)
                        };
                    }
                    ChunkProgress::Complete => return self.complete(response),
                },
                DecodeState::UntilEof(mut response) => {
                    self.append_body(&mut response, self.buffer.len())?;
                    if eof {
                        return self.complete(response);
                    }
                    self.state = DecodeState::UntilEof(response);
                    return Ok(None);
                }
                DecodeState::Complete => {
                    self.state = DecodeState::Complete;
                    return Ok(None);
                }
                DecodeState::Poisoned => {
                    return Err(Http1CodecError::MalformedResponse(
                        "decoder entered an invalid state".into(),
                    ));
                }
            }
        }
    }

    fn parse_head(&mut self) -> Result<HeadProgress, Http1CodecError> {
        let Some(end) = find_subsequence(&self.buffer, b"\r\n\r\n").map(|index| index + 4) else {
            let aggregate = self
                .head_bytes_seen
                .checked_add(self.buffer.len())
                .ok_or(Http1CodecError::Capacity("response header bytes"))?;
            if aggregate > self.max_header_bytes {
                return Err(Http1CodecError::Capacity("response header bytes"));
            }
            return Ok(HeadProgress::Incomplete);
        };
        self.head_bytes_seen = self
            .head_bytes_seen
            .checked_add(end)
            .ok_or(Http1CodecError::Capacity("response header bytes"))?;
        if self.head_bytes_seen > self.max_header_bytes {
            return Err(Http1CodecError::Capacity("response header bytes"));
        }

        let slot_limit = self.max_header_bytes / 4 + 1;
        let slots = self.max_headers.min(slot_limit);
        let mut parsed_headers = vec![httparse::EMPTY_HEADER; slots];
        let mut parsed = httparse::Response::new(&mut parsed_headers);
        let parsed_end = match parsed.parse(&self.buffer[..end]) {
            Ok(httparse::Status::Complete(parsed_end)) => parsed_end,
            Ok(httparse::Status::Partial) => {
                return Err(Http1CodecError::MalformedResponse(
                    "header terminator was not parseable".into(),
                ));
            }
            Err(httparse::Error::TooManyHeaders) => {
                return Err(Http1CodecError::Capacity("response header count"));
            }
            Err(error) => {
                return Err(Http1CodecError::MalformedResponse(format!(
                    "invalid response headers: {error}"
                )));
            }
        };
        if parsed_end != end {
            return Err(Http1CodecError::MalformedResponse(
                "response header parser stopped before the terminator".into(),
            ));
        }
        let version = parsed.version.ok_or_else(|| {
            Http1CodecError::MalformedResponse("response has no HTTP version".into())
        })?;
        let status = parsed.code.ok_or_else(|| {
            Http1CodecError::MalformedResponse("response has no status code".into())
        })?;
        let status_text = parsed.reason.unwrap_or_default().to_owned();
        if status_text.len() > self.max_status_text_bytes {
            return Err(Http1CodecError::Capacity("response status text bytes"));
        }
        let headers = parsed
            .headers
            .iter()
            .map(|header| ApiHeader::new(header.name, latin1_to_string(header.value)))
            .collect::<Vec<_>>();
        self.buffer.drain(..end);

        if (100..200).contains(&status) && status != 101 {
            return Ok(HeadProgress::Informational);
        }
        let response = Http1Response {
            version,
            status,
            status_text,
            headers,
            body: Vec::new(),
        };
        let mode = body_mode(&response, self.request_is_head)?;
        Ok(HeadProgress::Final(response, mode))
    }

    fn process_chunked(
        &mut self,
        response: &mut Http1Response,
        phase: &mut ChunkPhase,
    ) -> Result<ChunkProgress, Http1CodecError> {
        loop {
            match *phase {
                ChunkPhase::Size => {
                    let Some(end) = find_subsequence(&self.buffer, b"\r\n") else {
                        if self.buffer.len() > MAX_CHUNK_SIZE_LINE_BYTES {
                            return Err(Http1CodecError::Capacity("chunk size line bytes"));
                        }
                        return Ok(ChunkProgress::Incomplete);
                    };
                    if end > MAX_CHUNK_SIZE_LINE_BYTES {
                        return Err(Http1CodecError::Capacity("chunk size line bytes"));
                    }
                    let line = &self.buffer[..end];
                    let size_bytes = line.split(|byte| *byte == b';').next().unwrap_or_default();
                    let size_bytes = trim_ascii(size_bytes);
                    if size_bytes.is_empty() || !size_bytes.iter().all(u8::is_ascii_hexdigit) {
                        return Err(Http1CodecError::MalformedResponse(
                            "invalid chunk size".into(),
                        ));
                    }
                    let size_text = std::str::from_utf8(size_bytes).map_err(|_| {
                        Http1CodecError::MalformedResponse("invalid chunk size".into())
                    })?;
                    let size = usize::from_str_radix(size_text, 16).map_err(|_| {
                        Http1CodecError::MalformedResponse("chunk size overflow".into())
                    })?;
                    let next_body_len = response
                        .body
                        .len()
                        .checked_add(size)
                        .ok_or(Http1CodecError::Capacity("response body bytes"))?;
                    if next_body_len > self.max_body_bytes {
                        return Err(Http1CodecError::Capacity("response body bytes"));
                    }
                    self.buffer.drain(..end + 2);
                    *phase = if size == 0 {
                        ChunkPhase::Trailers
                    } else {
                        ChunkPhase::Data(size)
                    };
                }
                ChunkPhase::Data(remaining) => {
                    let take = remaining.min(self.buffer.len());
                    self.append_body(response, take)?;
                    let remaining = remaining - take;
                    if remaining != 0 {
                        *phase = ChunkPhase::Data(remaining);
                        return Ok(ChunkProgress::Incomplete);
                    }
                    *phase = ChunkPhase::DataCrlf;
                }
                ChunkPhase::DataCrlf => {
                    if self.buffer.len() < 2 {
                        return Ok(ChunkProgress::Incomplete);
                    }
                    if &self.buffer[..2] != b"\r\n" {
                        return Err(Http1CodecError::MalformedResponse(
                            "chunk data is not followed by CRLF".into(),
                        ));
                    }
                    self.buffer.drain(..2);
                    *phase = ChunkPhase::Size;
                }
                ChunkPhase::Trailers => {
                    if self.buffer.starts_with(b"\r\n") {
                        self.buffer.drain(..2);
                        return Ok(ChunkProgress::Complete);
                    }
                    let Some(end) =
                        find_subsequence(&self.buffer, b"\r\n\r\n").map(|index| index + 4)
                    else {
                        self.ensure_trailer_limit(self.buffer.len())?;
                        return Ok(ChunkProgress::Incomplete);
                    };
                    self.ensure_trailer_limit(end)?;
                    validate_trailers(&self.buffer[..end], self.max_headers)?;
                    self.buffer.drain(..end);
                    return Ok(ChunkProgress::Complete);
                }
            }
        }
    }

    fn append_body(
        &mut self,
        response: &mut Http1Response,
        bytes: usize,
    ) -> Result<(), Http1CodecError> {
        let next_len = response
            .body
            .len()
            .checked_add(bytes)
            .ok_or(Http1CodecError::Capacity("response body bytes"))?;
        if next_len > self.max_body_bytes {
            return Err(Http1CodecError::Capacity("response body bytes"));
        }
        response.body.extend_from_slice(&self.buffer[..bytes]);
        self.buffer.drain(..bytes);
        Ok(())
    }

    fn ensure_trailer_limit(&self, trailer_bytes: usize) -> Result<(), Http1CodecError> {
        let aggregate = self
            .head_bytes_seen
            .checked_add(trailer_bytes)
            .ok_or(Http1CodecError::Capacity("response header bytes"))?;
        if aggregate > self.max_header_bytes {
            Err(Http1CodecError::Capacity("response header bytes"))
        } else {
            Ok(())
        }
    }

    fn complete(
        &mut self,
        response: Http1Response,
    ) -> Result<Option<Http1ResponseEvent>, Http1CodecError> {
        if !self.buffer.is_empty() {
            return Err(Http1CodecError::TrailingBytes);
        }
        self.state = DecodeState::Complete;
        Ok(Some(Http1ResponseEvent::Complete(response)))
    }
}

enum HeadProgress {
    Incomplete,
    Informational,
    Final(Http1Response, BodyMode),
}

#[derive(Clone, Copy)]
enum BodyMode {
    None,
    Fixed(usize),
    Chunked,
    UntilEof,
}

enum ChunkProgress {
    Incomplete,
    Complete,
}

fn body_mode(response: &Http1Response, request_is_head: bool) -> Result<BodyMode, Http1CodecError> {
    if request_is_head
        || (100..200).contains(&response.status)
        || response.status == 204
        || response.status == 304
    {
        return Ok(BodyMode::None);
    }

    let transfer_encodings = response
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("transfer-encoding"))
        .flat_map(|header| header.value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !transfer_encodings.is_empty() {
        return if transfer_encodings
            .last()
            .is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
        {
            Ok(BodyMode::Chunked)
        } else {
            Ok(BodyMode::UntilEof)
        };
    }

    let content_lengths = response
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("content-length"))
        .flat_map(|header| header.value.split(','))
        .map(str::trim)
        .collect::<Vec<_>>();
    if content_lengths.is_empty() {
        return Ok(BodyMode::UntilEof);
    }
    let mut parsed = None;
    for value in content_lengths {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Http1CodecError::MalformedResponse(
                "invalid Content-Length".into(),
            ));
        }
        let value = value
            .parse::<usize>()
            .map_err(|_| Http1CodecError::MalformedResponse("Content-Length overflow".into()))?;
        if parsed.is_some_and(|previous| previous != value) {
            return Err(Http1CodecError::MalformedResponse(
                "conflicting Content-Length values".into(),
            ));
        }
        parsed = Some(value);
    }
    let length = parsed.unwrap_or_default();
    Ok(BodyMode::Fixed(length))
}

fn validate_trailers(bytes: &[u8], max_headers: usize) -> Result<(), Http1CodecError> {
    let content = bytes
        .strip_suffix(b"\r\n\r\n")
        .ok_or_else(|| Http1CodecError::MalformedResponse("invalid trailer terminator".into()))?;
    let mut count = 0usize;
    for line in content.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        count = count
            .checked_add(1)
            .ok_or(Http1CodecError::Capacity("response header count"))?;
        if count > max_headers {
            return Err(Http1CodecError::Capacity("response header count"));
        }
        let Some(colon) = line.iter().position(|byte| *byte == b':') else {
            return Err(Http1CodecError::MalformedResponse(
                "trailer has no colon".into(),
            ));
        };
        if colon == 0 || !line[..colon].iter().copied().all(is_token_byte) {
            return Err(Http1CodecError::MalformedResponse(
                "invalid trailer name".into(),
            ));
        }
        if !line[colon + 1..].iter().copied().all(is_header_value_byte) {
            return Err(Http1CodecError::MalformedResponse(
                "invalid trailer value".into(),
            ));
        }
    }
    Ok(())
}

fn latin1_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

const fn is_header_value_char(character: char) -> bool {
    character == '\t'
        || (character >= ' ' && character <= '~')
        || (character >= '\u{80}' && character <= '\u{ff}')
}

const fn is_header_value_byte(byte: u8) -> bool {
    byte == b'\t' || (byte >= 0x20 && byte <= 0x7e) || byte >= 0x80
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_api_request::{ApiHeader, TransportRequest};
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::{Http1CodecError, Http1ResponseDecoder, Http1ResponseEvent, encode_request};

    fn request(method: &str, url: &str) -> TransportRequest {
        TransportRequest {
            method: method.into(),
            url: Url::parse(url).expect("test URL should parse"),
            headers: vec![ApiHeader::new("x-Custom-Case", "caf\u{e9}")],
            body: None,
            timeout: Some(Duration::from_secs(1)),
            ignore_https_errors: false,
            cookie_store: None,
            authorization_provider: None,
            max_request_header_bytes: 4096,
            max_response_body_bytes: 1024,
            max_response_headers: 16,
            max_response_header_bytes: 4096,
            max_response_status_text_bytes: 128,
            max_response_log_entries: 8,
            max_response_log_bytes: 1024,
            max_redirects: 0,
            max_retries: 0,
            lookup_addresses: None,
            cancellation: CancellationToken::new(),
        }
    }

    #[test]
    fn request_encoder_preserves_headers_and_adds_connection_ownership() {
        // Arrange
        let mut request = request("POST", "http://example.test:8080/a/b?q=hello#ignored");
        request.body = Some(b"body".to_vec());
        request.headers.push(ApiHeader::new("Content-Length", "4"));

        // Act
        let encoded = encode_request(&request).expect("request should encode");

        // Assert
        assert_eq!(
            encoded,
            b"POST /a/b?q=hello HTTP/1.1\r\nx-Custom-Case: caf\xe9\r\nContent-Length: 4\r\nHost: example.test:8080\r\nConnection: close\r\n\r\nbody"
        );
    }

    #[test]
    fn response_preserves_header_case_duplicates_and_latin1() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));
        let bytes = b"HTTP/1.1 200 Fine\r\nContent-Type: text/plain\r\nSet-Cookie: a=1\r\nSet-Cookie: b=\xe9\r\nContent-Length: 2\r\n\r\nok";

        // Act
        let response = decoder
            .feed(bytes)
            .expect("response should parse")
            .expect("response should be complete");

        // Assert
        assert_eq!(response.status, 200);
        assert_eq!(response.status_text, "Fine");
        assert_eq!(
            response.headers[0],
            ApiHeader::new("Content-Type", "text/plain")
        );
        assert_eq!(response.headers[1], ApiHeader::new("Set-Cookie", "a=1"));
        assert_eq!(
            response.headers[2],
            ApiHeader::new("Set-Cookie", "b=\u{e9}")
        );
        assert_eq!(response.body, b"ok");
    }

    #[test]
    fn event_api_yields_final_head_before_buffered_body() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));
        let bytes = b"HTTP/1.1 200 Fine\r\nContent-Length: 2\r\n\r\nok";

        // Act
        let head = decoder
            .feed_event(bytes)
            .expect("head should parse")
            .expect("head event should exist");
        let complete = decoder
            .feed_event(&[])
            .expect("buffered body should parse")
            .expect("completion event should exist");

        // Assert
        assert!(matches!(
            head,
            Http1ResponseEvent::Head(head) if head.status == 200 && head.status_text == "Fine"
        ));
        assert!(matches!(
            complete,
            Http1ResponseEvent::Complete(response) if response.body == b"ok"
        ));
    }

    #[test]
    fn event_api_exposes_redirect_head_before_oversized_body_failure() {
        // Arrange
        let mut input = request("GET", "http://example.test/");
        input.max_response_body_bytes = 1;
        let mut decoder = Http1ResponseDecoder::new(&input);
        let bytes = b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 100\r\n\r\nignored";

        // Act
        let head = decoder
            .feed_event(bytes)
            .expect("redirect head should parse")
            .expect("head event should exist");
        let body_error = decoder
            .feed_event(&[])
            .expect_err("oversized body should fail only when consumed");

        // Assert
        assert!(matches!(
            head,
            Http1ResponseEvent::Head(head) if head.status == 302
        ));
        assert_eq!(body_error, Http1CodecError::Capacity("response body bytes"));
    }

    #[test]
    fn fixed_length_response_accepts_every_fragment_boundary() {
        // Arrange
        let bytes = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";

        for split in 0..=bytes.len() {
            let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));

            // Act
            let first = decoder
                .feed(&bytes[..split])
                .expect("first fragment should parse");
            let second = decoder
                .feed(&bytes[split..])
                .expect("second fragment should parse");

            // Assert
            let response = first.or(second).expect("response should complete");
            assert_eq!(response.body, b"hello");
        }
    }

    #[test]
    fn chunked_response_accepts_fragments_and_discards_trailers() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));
        let fragments: [&[u8]; 5] = [
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r",
            b"\nWi",
            b"ki\r\n5;extension=yes\r\npe",
            b"dia\r\n0\r\nX-Trailer: yes\r\n",
            b"\r\n",
        ];
        let mut completed = None;

        // Act
        for fragment in fragments {
            completed = completed.or(decoder.feed(fragment).expect("chunk fragment should parse"));
        }

        // Assert
        let response = completed.expect("chunked response should complete");
        assert_eq!(response.body, b"Wikipedia");
        assert_eq!(response.headers.len(), 1);
    }

    #[test]
    fn chunked_response_accepts_every_fragment_boundary() {
        // Arrange
        let bytes = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\n\r\n";

        for split in 0..=bytes.len() {
            let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));

            // Act
            let first = decoder
                .feed(&bytes[..split])
                .expect("first fragment should parse");
            let second = decoder
                .feed(&bytes[split..])
                .expect("second fragment should parse");

            // Assert
            let response = first.or(second).expect("response should complete");
            assert_eq!(response.body, b"hi");
        }
    }

    #[test]
    fn informational_response_is_skipped_before_final_response() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("POST", "http://example.test/"));
        let bytes = b"HTTP/1.1 100 Continue\r\nX-Interim: yes\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";

        // Act
        let response = decoder
            .feed(bytes)
            .expect("responses should parse")
            .expect("final response should complete");

        // Assert
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ok");
        assert_eq!(
            response.headers,
            vec![ApiHeader::new("Content-Length", "2")]
        );
    }

    #[test]
    fn head_and_no_content_responses_never_read_a_body() {
        // Arrange
        let mut head_decoder = Http1ResponseDecoder::new(&request("HEAD", "http://example.test/"));
        let mut no_content_decoder =
            Http1ResponseDecoder::new(&request("GET", "http://example.test/"));

        // Act
        let head = head_decoder
            .feed(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n")
            .expect("HEAD response should parse")
            .expect("HEAD response should complete");
        let no_content = no_content_decoder
            .feed(b"HTTP/1.1 204 No Content\r\n\r\n")
            .expect("204 response should parse")
            .expect("204 response should complete");

        // Assert
        assert!(head.body.is_empty());
        assert!(no_content.body.is_empty());
    }

    #[test]
    fn close_delimited_response_completes_only_at_eof() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));

        // Act
        let before_eof = decoder
            .feed(b"HTTP/1.0 200 OK\r\nContent-Type: text/plain\r\n\r\nhello")
            .expect("response fragment should parse");
        let at_eof = decoder
            .finish()
            .expect("EOF should complete response")
            .expect("response should be returned");

        // Assert
        assert!(before_eof.is_none());
        assert_eq!(at_eof.body, b"hello");
    }

    #[test]
    fn configured_limits_reject_headers_body_chunks_and_trailers() {
        // Arrange
        let mut body_request = request("GET", "http://example.test/");
        body_request.max_response_body_bytes = 3;
        let mut body_decoder = Http1ResponseDecoder::new(&body_request);
        let mut chunk_decoder = Http1ResponseDecoder::new(&body_request);
        let mut header_request = request("GET", "http://example.test/");
        header_request.max_response_header_bytes = 20;
        let mut header_decoder = Http1ResponseDecoder::new(&header_request);
        let mut trailer_request = request("GET", "http://example.test/");
        trailer_request.max_response_header_bytes = 60;
        let mut trailer_decoder = Http1ResponseDecoder::new(&trailer_request);

        // Act
        let body_error = body_decoder
            .feed(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n")
            .expect_err("declared body should exceed the limit");
        let chunk_error = chunk_decoder
            .feed(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n")
            .expect_err("declared chunk should exceed the limit");
        let header_error = header_decoder
            .feed(b"HTTP/1.1 200 OK\r\nLong: header\r\n\r\n")
            .expect_err("headers should exceed the limit");
        let trailer_error = trailer_decoder
            .feed(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX: 1234567890\r\n\r\n",
            )
            .expect_err("headers plus trailers should exceed the limit");

        // Assert
        assert_eq!(body_error, Http1CodecError::Capacity("response body bytes"));
        assert_eq!(
            chunk_error,
            Http1CodecError::Capacity("response body bytes")
        );
        assert_eq!(
            header_error,
            Http1CodecError::Capacity("response header bytes")
        );
        assert_eq!(
            trailer_error,
            Http1CodecError::Capacity("response header bytes")
        );
    }

    #[test]
    fn truncated_fixed_and_chunked_responses_fail_at_eof() {
        // Arrange
        let mut fixed = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));
        let mut chunked = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));
        fixed
            .feed(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\na")
            .expect("partial fixed response should parse");
        chunked
            .feed(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\na")
            .expect("partial chunked response should parse");

        // Act
        let fixed_error = fixed
            .finish()
            .expect_err("fixed response should be truncated");
        let chunked_error = chunked
            .finish()
            .expect_err("chunked response should be truncated");

        // Assert
        assert_eq!(
            fixed_error,
            Http1CodecError::Truncated("fixed-length response body")
        );
        assert_eq!(
            chunked_error,
            Http1CodecError::Truncated("chunked response body")
        );
    }

    #[test]
    fn bytes_after_owned_exchange_are_rejected() {
        // Arrange
        let mut decoder = Http1ResponseDecoder::new(&request("GET", "http://example.test/"));

        // Act
        let error = decoder
            .feed(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nxextra")
            .expect_err("pipelined bytes should be rejected");

        // Assert
        assert_eq!(error, Http1CodecError::TrailingBytes);
    }
}
