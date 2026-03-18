use anyhow::{Result, bail, ensure};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use http::header::HeaderName;
use http::uri::PathAndQuery;
use http::{HeaderMap, HeaderValue, Method};
use uuid::Uuid;

pub type Headers = HeaderMap<HeaderValue>;

// ── Frame ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Frame {
    OpenStream {
        stream_id: Uuid,
        method: Method,
        path_and_query: PathAndQuery,
        headers: Headers,
        content_length: Option<u64>,
    },
    RequestBodyChunk {
        stream_id: Uuid,
        data: Bytes,
    },
    RequestBodyEnd {
        stream_id: Uuid,
    },
    ResponseHead {
        stream_id: Uuid,
        status: u16,
        headers: Headers,
    },
    ResponseBodyChunk {
        stream_id: Uuid,
        data: Bytes,
    },
    ResponseBodyEnd {
        stream_id: Uuid,
    },
    CancelStream {
        stream_id: Uuid,
    },
    ErrorStream {
        stream_id: Uuid,
        status: u16,
        message: String,
    },
}

impl Frame {
    pub fn stream_id(&self) -> Uuid {
        match self {
            Self::OpenStream { stream_id, .. }
            | Self::RequestBodyChunk { stream_id, .. }
            | Self::RequestBodyEnd { stream_id }
            | Self::ResponseHead { stream_id, .. }
            | Self::ResponseBodyChunk { stream_id, .. }
            | Self::ResponseBodyEnd { stream_id }
            | Self::CancelStream { stream_id }
            | Self::ErrorStream { stream_id, .. } => *stream_id,
        }
    }
}

// ── Tags ─────────────────────────────────────────────────────────────────

const TAG_OPEN_STREAM: u8 = 0;
const TAG_REQUEST_BODY_CHUNK: u8 = 1;
const TAG_REQUEST_BODY_END: u8 = 2;
const TAG_RESPONSE_HEAD: u8 = 3;
const TAG_RESPONSE_BODY_CHUNK: u8 = 4;
const TAG_RESPONSE_BODY_END: u8 = 5;
const TAG_CANCEL_STREAM: u8 = 6;
const TAG_ERROR_STREAM: u8 = 7;

// ── Encode ───────────────────────────────────────────────────────────────

pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>> {
    let mut buf = BytesMut::with_capacity(256);

    match frame {
        Frame::OpenStream {
            stream_id,
            method,
            path_and_query,
            headers,
            content_length,
        } => {
            // TAG
            buf.put_u8(TAG_OPEN_STREAM);

            // Stream ID
            put_uuid(&mut buf, stream_id);

            // Method
            put_str(&mut buf, method.as_str());

            // Path and Query
            put_str(&mut buf, path_and_query.as_str());

            // Length
            put_opt_u64(&mut buf, content_length);

            // Headers
            put_headers(&mut buf, headers);
        }
        Frame::RequestBodyChunk { stream_id, data } => {
            // Tag
            buf.put_u8(TAG_REQUEST_BODY_CHUNK);

            // Stream ID
            put_uuid(&mut buf, stream_id);

            // Data chunk
            buf.put_slice(data);
        }
        Frame::RequestBodyEnd { stream_id } => {
            // Tag
            buf.put_u8(TAG_REQUEST_BODY_END);

            // Stream ID
            put_uuid(&mut buf, stream_id);
        }
        Frame::ResponseHead {
            stream_id,
            status,
            headers,
        } => {
            // TAG
            buf.put_u8(TAG_RESPONSE_HEAD);

            // Stream ID
            put_uuid(&mut buf, stream_id);

            // Status
            buf.put_u16(*status);

            // Headers
            put_headers(&mut buf, headers);
        }
        Frame::ResponseBodyChunk { stream_id, data } => {
            // Tag
            buf.put_u8(TAG_RESPONSE_BODY_CHUNK);

            // Stream ID
            put_uuid(&mut buf, stream_id);

            // Data chunk
            buf.put_slice(data);
        }
        Frame::ResponseBodyEnd { stream_id } => {
            // Tag
            buf.put_u8(TAG_RESPONSE_BODY_END);

            // Stream ID
            put_uuid(&mut buf, stream_id);
        }
        Frame::CancelStream { stream_id } => {
            // Tag
            buf.put_u8(TAG_CANCEL_STREAM);

            // Stream ID
            put_uuid(&mut buf, stream_id);
        }
        Frame::ErrorStream {
            stream_id,
            status,
            message,
        } => {
            // Tag
            buf.put_u8(TAG_ERROR_STREAM);

            // Stream ID
            put_uuid(&mut buf, stream_id);

            // Status
            buf.put_u16(*status);

            // Error message
            buf.put_slice(message.as_bytes());
        }
    }

    Ok(buf.to_vec())
}

// ── Decode ───────────────────────────────────────────────────────────────

pub fn decode_frame(bytes: &[u8]) -> Result<Frame> {
    ensure!(!bytes.is_empty(), "empty frame");
    let mut buf = bytes;

    let tag = buf.get_u8();
    let stream_id = get_uuid(&mut buf)?;

    match tag {
        TAG_OPEN_STREAM => {
            let method = get_str(&mut buf)?
                .parse::<Method>()
                .map_err(|e| anyhow::anyhow!("invalid method: {e}"))?;
            let path_and_query = get_str(&mut buf)?
                .parse::<PathAndQuery>()
                .map_err(|e| anyhow::anyhow!("invalid path_and_query: {e}"))?;
            let content_length = get_opt_u64(&mut buf)?;
            let headers = get_headers(&mut buf)?;
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in OpenStream"
            );
            Ok(Frame::OpenStream {
                stream_id,
                method,
                path_and_query,
                headers,
                content_length,
            })
        }
        TAG_REQUEST_BODY_CHUNK => Ok(Frame::RequestBodyChunk {
            stream_id,
            data: Bytes::copy_from_slice(buf),
        }),
        TAG_REQUEST_BODY_END => {
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in RequestBodyEnd"
            );
            Ok(Frame::RequestBodyEnd { stream_id })
        }
        TAG_RESPONSE_HEAD => {
            ensure!(buf.remaining() >= 2, "truncated response head");
            let status = buf.get_u16();
            let headers = get_headers(&mut buf)?;
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in ResponseHead"
            );
            Ok(Frame::ResponseHead {
                stream_id,
                status,
                headers,
            })
        }
        TAG_RESPONSE_BODY_CHUNK => Ok(Frame::ResponseBodyChunk {
            stream_id,
            data: Bytes::copy_from_slice(buf),
        }),
        TAG_RESPONSE_BODY_END => {
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in ResponseBodyEnd"
            );
            Ok(Frame::ResponseBodyEnd { stream_id })
        }
        TAG_CANCEL_STREAM => {
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in CancelStream"
            );
            Ok(Frame::CancelStream { stream_id })
        }
        TAG_ERROR_STREAM => {
            ensure!(buf.remaining() >= 2, "truncated error stream");
            let status = buf.get_u16();
            let message = std::str::from_utf8(buf)?.to_owned();
            buf.advance(buf.len());
            ensure!(
                !buf.has_remaining(),
                "unexpected trailing bytes in ErrorStream"
            );
            Ok(Frame::ErrorStream {
                stream_id,
                status,
                message,
            })
        }
        _ => bail!("unknown frame tag: {tag}"),
    }
}

// ── Primitives ───────────────────────────────────────────────────────────

fn put_uuid(buf: &mut BytesMut, id: &Uuid) {
    buf.put_slice(id.as_bytes());
}

fn get_uuid(buf: &mut &[u8]) -> Result<Uuid> {
    ensure!(buf.remaining() >= 16, "truncated uuid");
    let mut bytes = [0u8; 16];

    buf.copy_to_slice(&mut bytes);
    Ok(Uuid::from_bytes(bytes))
}

fn put_str(buf: &mut BytesMut, s: &str) {
    buf.put_u16(s.len() as u16);
    buf.put_slice(s.as_bytes());
}

fn get_str<'a>(buf: &mut &'a [u8]) -> Result<&'a str> {
    ensure!(buf.remaining() >= 2, "truncated string length");
    let len = buf.get_u16() as usize;

    ensure!(buf.remaining() >= len, "truncated string data");
    let s = std::str::from_utf8(&buf[..len])?;

    buf.advance(len);

    Ok(s)
}

fn put_opt_u64(buf: &mut BytesMut, val: &Option<u64>) {
    match val {
        Some(v) => {
            buf.put_u8(1);
            buf.put_u64(*v);
        }
        None => buf.put_u8(0),
    }
}

fn get_opt_u64(buf: &mut &[u8]) -> Result<Option<u64>> {
    ensure!(buf.remaining() >= 1, "truncated option tag");
    match buf.get_u8() {
        0 => Ok(None),
        1 => {
            ensure!(buf.remaining() >= 8, "truncated option value");
            Ok(Some(buf.get_u64()))
        }
        t => bail!("invalid option tag: {t}"),
    }
}

fn put_headers(buf: &mut BytesMut, headers: &Headers) {
    buf.put_u16(headers.len() as u16);
    for (name, value) in headers {
        put_str(buf, name.as_str());
        let v = value.as_bytes();
        buf.put_u16(v.len() as u16);
        buf.put_slice(v);
    }
}

fn get_headers(buf: &mut &[u8]) -> Result<Headers> {
    ensure!(buf.remaining() >= 2, "truncated headers count");
    let count = buf.get_u16() as usize;

    let mut headers = HeaderMap::with_capacity(count);
    for _ in 0..count {
        let name = get_str(buf)?;

        ensure!(buf.remaining() >= 2, "truncated header value length");
        let vlen = buf.get_u16() as usize;

        ensure!(buf.remaining() >= vlen, "truncated header value");
        let value = &buf[..vlen];

        buf.advance(vlen);
        headers.append(HeaderName::try_from(name)?, HeaderValue::from_bytes(value)?);
    }

    Ok(headers)
}
