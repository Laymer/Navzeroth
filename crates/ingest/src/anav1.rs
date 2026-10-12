//! The `ANAV1:` text transport (plan section 7.1). The grammar is normative here and
//! documented in `schemas/v1/README.md`:
//!
//! ```text
//! transport = ws* "ANAV1:" document ws*
//! ws        = SP | HT | LF | CR
//! document  = one JSON exchange envelope object, UTF-8, no byte order mark
//! ```
//!
//! The prefix is case sensitive and followed directly by the JSON. There is no
//! compression, base64 or chunking. Anything other than ASCII whitespace before
//! the prefix or after the JSON (a byte order mark, a second prefix, trailing
//! text) is rejected. The decoded document is bounded to 4 MiB and JSON
//! nesting to 32 before any JSON is parsed. Clipboards are never polled; a caller
//! supplies the text explicitly.
use crate::error::{invalid, too_large};
use azeroth_core::{
    contract::{AnyExchange, MAX_ADDON_JSON_DEPTH, MAX_ENVELOPE_BYTES},
    error::Result,
};

/// The case-sensitive transport prefix.
pub const PREFIX: &str = "ANAV1:";
/// Largest decoded document accepted by any text transport (plan section 7.1).
/// Reference datasets may exceed this in files, but not through this transport.
pub const MAX_DOCUMENT_BYTES: usize = MAX_ENVELOPE_BYTES;
/// Largest accepted raw text: the document, the prefix and generous whitespace slack.
pub const MAX_TEXT_BYTES: usize = MAX_ENVELOPE_BYTES + 1024;
/// JSON nesting limit applied to every document before parsing.
pub const MAX_JSON_DEPTH: usize = MAX_ADDON_JSON_DEPTH;

fn is_transport_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// Encode a validated exchange as `ANAV1:` plus its compact canonical JSON, with
/// no trailing newline. Fails with `IMPORT_TOO_LARGE` above the 4 MiB ceiling.
pub fn encode_text(exchange: &AnyExchange) -> Result<String> {
    let json = serde_json::to_string(exchange)
        .map_err(|_| invalid("Exchange document cannot be serialized"))?;
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large("Exchange document exceeds the transport limit"));
    }
    let mut text = String::with_capacity(PREFIX.len() + json.len());
    text.push_str(PREFIX);
    text.push_str(&json);
    Ok(text)
}

/// Strip the transport framing and return the JSON document slice. Checks, in
/// order: raw size, prefix, document size, first byte, JSON nesting depth.
/// The JSON itself is parsed and validated by the caller (see `import::normalize`).
pub fn decode_text(text: &str) -> Result<&str> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(too_large("Transport text exceeds its size limit"));
    }
    let Some(document) = text
        .trim_matches(is_transport_whitespace)
        .strip_prefix(PREFIX)
    else {
        return Err(invalid(
            "Transport text does not start with the ANAV1 prefix",
        ));
    };
    check_document(document)?;
    Ok(document)
}

/// Size, shape and nesting checks for a document that is about to be parsed.
/// Shared by the text path and the SavedVariables path so both enforce the same
/// ceilings before JSON parsing.
pub(crate) fn check_document(document: &str) -> Result<()> {
    if document.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large("Exchange document exceeds the transport limit"));
    }
    if !document.starts_with('{') {
        return Err(invalid("ANAV1 payload must be a JSON object"));
    }
    if !json_depth_within(document, MAX_JSON_DEPTH) {
        return Err(invalid("Exchange JSON nesting depth exceeded"));
    }
    Ok(())
}

/// True when bracket nesting never exceeds `max`, ignoring brackets in strings.
/// Linear time, no recursion.
pub fn json_depth_within(json: &str, max: usize) -> bool {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for b in json.bytes() {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max {
                    return false;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    true
}
