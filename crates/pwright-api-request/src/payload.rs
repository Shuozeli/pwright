use std::sync::atomic::{AtomicU64, Ordering};

use url::Url;

use crate::ApiRequestError;

const MULTIPART_BOUNDARY_PREFIX: &str = "----WebKitFormBoundary";
const MULTIPART_BOUNDARY_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789AB";
const MAX_BOUNDARY_COLLISION_ATTEMPTS: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiNameValue {
    pub name: String,
    pub value: String,
}

impl ApiNameValue {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ApiQuery {
    #[default]
    None,
    Encoded(String),
    Pairs(Vec<ApiNameValue>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiFilePayload {
    pub name: String,
    pub mime_type: Option<String>,
    pub buffer: Vec<u8>,
}

impl ApiFilePayload {
    pub fn new(name: impl Into<String>, buffer: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            mime_type: None,
            buffer: buffer.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiMultipartField {
    Text(ApiNameValue),
    File { name: String, file: ApiFilePayload },
}

impl ApiMultipartField {
    pub fn text(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Text(ApiNameValue::new(name, value))
    }

    pub fn file(name: impl Into<String>, file: ApiFilePayload) -> Self {
        Self::File {
            name: name.into(),
            file,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ApiRequestBody {
    #[default]
    None,
    Bytes(Vec<u8>),
    Json(String),
    Form(Vec<ApiNameValue>),
    Multipart(Vec<ApiMultipartField>),
}

pub(crate) struct SerializedBody {
    pub(crate) bytes: Option<Vec<u8>>,
    pub(crate) content_type: Option<String>,
}

pub(crate) fn apply_query(
    url: &mut Url,
    query: ApiQuery,
    max_parameters: usize,
    max_url_bytes: usize,
) -> Result<(), ApiRequestError> {
    match query {
        ApiQuery::None => {}
        ApiQuery::Encoded(encoded) => {
            if encoded.len() > max_url_bytes {
                return Err(ApiRequestError::Capacity("request query bytes".into()));
            }
            if !encoded.is_empty() {
                url.set_query(Some(encoded.strip_prefix('?').unwrap_or(&encoded)));
            }
        }
        ApiQuery::Pairs(pairs) => {
            if pairs.len() > max_parameters {
                return Err(ApiRequestError::Capacity(
                    "request query parameter count".into(),
                ));
            }
            for pair in pairs {
                if pair.name.len() > max_url_bytes || pair.value.len() > max_url_bytes {
                    return Err(ApiRequestError::Capacity("request query bytes".into()));
                }
                url.query_pairs_mut().append_pair(&pair.name, &pair.value);
                if url.as_str().len() > max_url_bytes {
                    return Err(ApiRequestError::Capacity("request URL bytes".into()));
                }
            }
        }
    }
    if url.as_str().len() > max_url_bytes {
        return Err(ApiRequestError::Capacity("request URL bytes".into()));
    }
    Ok(())
}

pub(crate) fn serialize_body(
    body: ApiRequestBody,
    max_fields: usize,
    max_body_bytes: usize,
    next_boundary_id: &AtomicU64,
) -> Result<SerializedBody, ApiRequestError> {
    match body {
        ApiRequestBody::None => Ok(SerializedBody {
            bytes: None,
            content_type: None,
        }),
        ApiRequestBody::Bytes(bytes) => {
            ensure_body_limit(bytes.len(), max_body_bytes)?;
            Ok(SerializedBody {
                bytes: Some(bytes),
                content_type: Some("application/octet-stream".into()),
            })
        }
        ApiRequestBody::Json(json) => {
            ensure_body_limit(json.len(), max_body_bytes)?;
            Ok(SerializedBody {
                bytes: Some(json.into_bytes()),
                content_type: Some("application/json".into()),
            })
        }
        ApiRequestBody::Form(fields) => {
            ensure_field_count(fields.len(), max_fields)?;
            let bytes = serialize_form(fields, max_body_bytes)?;
            Ok(SerializedBody {
                bytes: Some(bytes),
                content_type: Some("application/x-www-form-urlencoded".into()),
            })
        }
        ApiRequestBody::Multipart(fields) => {
            ensure_field_count(fields.len(), max_fields)?;
            let boundary = next_boundary(fields.as_slice(), next_boundary_id)?;
            let bytes = serialize_multipart(fields, &boundary, max_body_bytes)?;
            Ok(SerializedBody {
                bytes: Some(bytes),
                content_type: Some(format!("multipart/form-data; boundary={boundary}")),
            })
        }
    }
}

fn serialize_form(
    fields: Vec<ApiNameValue>,
    max_body_bytes: usize,
) -> Result<Vec<u8>, ApiRequestError> {
    let mut output = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        if index != 0 {
            append_bounded(&mut output, b"&", max_body_bytes)?;
        }
        append_form_component(&mut output, field.name.as_bytes(), max_body_bytes)?;
        append_bounded(&mut output, b"=", max_body_bytes)?;
        append_form_component(&mut output, field.value.as_bytes(), max_body_bytes)?;
    }
    Ok(output)
}

fn append_form_component(
    output: &mut Vec<u8>,
    value: &[u8],
    max_body_bytes: usize,
) -> Result<(), ApiRequestError> {
    for encoded in url::form_urlencoded::byte_serialize(value) {
        append_bounded(output, encoded.as_bytes(), max_body_bytes)?;
    }
    Ok(())
}

fn next_boundary(
    fields: &[ApiMultipartField],
    next_boundary_id: &AtomicU64,
) -> Result<String, ApiRequestError> {
    for _ in 0..MAX_BOUNDARY_COLLISION_ATTEMPTS {
        let id = next_boundary_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| ApiRequestError::Capacity("multipart boundary identity space".into()))?;
        let boundary = boundary_for_id(id);
        if !multipart_contains(fields, boundary.as_bytes()) {
            return Ok(boundary);
        }
    }
    Err(ApiRequestError::Capacity(
        "multipart boundary collision attempts".into(),
    ))
}

fn boundary_for_id(mut id: u64) -> String {
    let mut suffix = [b'A'; 16];
    for byte in suffix.iter_mut().rev() {
        *byte = MULTIPART_BOUNDARY_ALPHABET[(id & 63) as usize];
        id >>= 6;
    }
    let suffix = String::from_utf8_lossy(&suffix);
    format!("{MULTIPART_BOUNDARY_PREFIX}{suffix}")
}

fn multipart_contains(fields: &[ApiMultipartField], needle: &[u8]) -> bool {
    fields.iter().any(|field| match field {
        ApiMultipartField::Text(value) => {
            contains(value.name.as_bytes(), needle) || contains(value.value.as_bytes(), needle)
        }
        ApiMultipartField::File { name, file } => {
            contains(name.as_bytes(), needle)
                || contains(file.name.as_bytes(), needle)
                || file
                    .mime_type
                    .as_deref()
                    .is_some_and(|value| contains(value.as_bytes(), needle))
                || contains(&file.buffer, needle)
        }
    })
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn serialize_multipart(
    fields: Vec<ApiMultipartField>,
    boundary: &str,
    max_body_bytes: usize,
) -> Result<Vec<u8>, ApiRequestError> {
    let mut output = Vec::new();
    for field in fields {
        append_bounded(&mut output, b"--", max_body_bytes)?;
        append_bounded(&mut output, boundary.as_bytes(), max_body_bytes)?;
        append_bounded(
            &mut output,
            b"\r\ncontent-disposition: form-data; name=\"",
            max_body_bytes,
        )?;
        match field {
            ApiMultipartField::Text(value) => {
                append_bounded(&mut output, value.name.as_bytes(), max_body_bytes)?;
                append_bounded(&mut output, b"\"\r\n\r\n", max_body_bytes)?;
                append_bounded(&mut output, value.value.as_bytes(), max_body_bytes)?;
            }
            ApiMultipartField::File { name, file } => {
                append_bounded(&mut output, name.as_bytes(), max_body_bytes)?;
                append_bounded(&mut output, b"\"; filename=\"", max_body_bytes)?;
                append_bounded(&mut output, file.name.as_bytes(), max_body_bytes)?;
                append_bounded(&mut output, b"\"\r\ncontent-type: ", max_body_bytes)?;
                let mime_type = file
                    .mime_type
                    .or_else(|| {
                        mime_guess::from_path(&file.name)
                            .first_raw()
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "application/octet-stream".into());
                append_bounded(&mut output, mime_type.as_bytes(), max_body_bytes)?;
                append_bounded(&mut output, b"\r\n\r\n", max_body_bytes)?;
                append_bounded(&mut output, &file.buffer, max_body_bytes)?;
            }
        }
        append_bounded(&mut output, b"\r\n", max_body_bytes)?;
    }
    append_bounded(&mut output, b"--", max_body_bytes)?;
    append_bounded(&mut output, boundary.as_bytes(), max_body_bytes)?;
    append_bounded(&mut output, b"--\r\n", max_body_bytes)?;
    Ok(output)
}

fn append_bounded(
    output: &mut Vec<u8>,
    value: &[u8],
    max_body_bytes: usize,
) -> Result<(), ApiRequestError> {
    let next_len = output
        .len()
        .checked_add(value.len())
        .ok_or_else(|| ApiRequestError::Capacity("request body bytes".into()))?;
    ensure_body_limit(next_len, max_body_bytes)?;
    output.extend_from_slice(value);
    Ok(())
}

fn ensure_field_count(count: usize, max_fields: usize) -> Result<(), ApiRequestError> {
    if count > max_fields {
        Err(ApiRequestError::Capacity("request body field count".into()))
    } else {
        Ok(())
    }
}

fn ensure_body_limit(bytes: usize, max_body_bytes: usize) -> Result<(), ApiRequestError> {
    if bytes > max_body_bytes {
        Err(ApiRequestError::Capacity("request body bytes".into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use url::Url;

    use super::{
        ApiFilePayload, ApiMultipartField, ApiNameValue, ApiQuery, ApiRequestBody, apply_query,
        serialize_body,
    };

    #[test]
    fn query_pairs_append_and_encoded_query_replaces() {
        // Arrange
        let mut paired = Url::parse("https://example.test/path?existing=yes").unwrap();
        let mut encoded = paired.clone();
        let mut empty = paired.clone();

        // Act
        apply_query(
            &mut paired,
            ApiQuery::Pairs(vec![
                ApiNameValue::new("name", "John Doe"),
                ApiNameValue::new("name", "Jane"),
            ]),
            8,
            1024,
        )
        .unwrap();
        apply_query(
            &mut encoded,
            ApiQuery::Encoded("?raw=hello%20world".into()),
            8,
            1024,
        )
        .unwrap();
        apply_query(&mut empty, ApiQuery::Encoded(String::new()), 8, 1024).unwrap();

        // Assert
        assert_eq!(
            paired.as_str(),
            "https://example.test/path?existing=yes&name=John+Doe&name=Jane"
        );
        assert_eq!(
            encoded.as_str(),
            "https://example.test/path?raw=hello%20world"
        );
        assert_eq!(empty.as_str(), "https://example.test/path?existing=yes");
    }

    #[test]
    fn form_serialization_preserves_repeated_names_and_encoding() {
        // Arrange
        let body = ApiRequestBody::Form(vec![
            ApiNameValue::new("name", "John Doe"),
            ApiNameValue::new("name", "a+b"),
        ]);

        // Act
        let serialized = serialize_body(body, 8, 1024, &AtomicU64::new(1)).unwrap();

        // Assert
        assert_eq!(
            serialized.content_type.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(serialized.bytes.unwrap(), b"name=John+Doe&name=a%2Bb");
    }

    #[test]
    fn multipart_serialization_preserves_order_and_infers_mime_type() {
        // Arrange
        let body = ApiRequestBody::Multipart(vec![
            ApiMultipartField::text("firstName", "John"),
            ApiMultipartField::file(
                "file",
                ApiFilePayload::new("data.json", br#"{"ok":true}"#.to_vec()),
            ),
            ApiMultipartField::text("firstName", "Doe"),
        ]);

        // Act
        let serialized = serialize_body(body, 8, 4096, &AtomicU64::new(1)).unwrap();

        // Assert
        let content_type = serialized.content_type.unwrap();
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .unwrap();
        let text = String::from_utf8(serialized.bytes.unwrap()).unwrap();
        assert!(text.starts_with(&format!("--{boundary}\r\n")));
        assert!(text.ends_with(&format!("--{boundary}--\r\n")));
        assert!(text.contains("filename=\"data.json\"\r\ncontent-type: application/json"));
        let first = text.find("\r\n\r\nJohn\r\n").unwrap();
        let file = text.find("filename=\"data.json\"").unwrap();
        let repeated = text.rfind("\r\n\r\nDoe\r\n").unwrap();
        assert!(first < file && file < repeated);
    }

    #[test]
    fn serialized_body_never_exceeds_limit() {
        // Arrange
        let body = ApiRequestBody::Form(vec![ApiNameValue::new("value", "%%%")]);

        // Act
        let result = serialize_body(body, 8, 10, &AtomicU64::new(1));

        // Assert
        assert!(result.is_err());
    }
}
