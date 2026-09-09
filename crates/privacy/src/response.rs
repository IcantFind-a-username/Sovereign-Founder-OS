//! The response boundary: what comes back from a public model is untrusted.
//!
//! Provider bytes are `RestrictedDerived` by default and may influence
//! nothing but their own text. They cannot widen a scope, name a recipient,
//! change a purpose, or authorize anything. Validation happens before
//! rehydration, and rehydration only puts back values the compiler withheld —
//! it can never introduce a value the model was not standing in for.

use crate::compile::PublicJob;
use crate::value::TrustedValue;

/// Ceiling on a provider response, independent of what any provider claims.
pub const MAX_RESPONSE_CHARS: usize = 24_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResponseError {
    #[error("the job expired before its response was accepted")]
    JobExpired,
    #[error("the response is empty")]
    Empty,
    #[error("the response is {actual} characters; the maximum is {maximum}")]
    TooLarge { actual: usize, maximum: usize },
    #[error("the response contains control characters")]
    ControlCharacters,
    #[error("the response uses placeholder `{0}`, which this job never sent")]
    UnknownPlaceholder(String),
}

/// A provider response that passed validation and had its placeholders put
/// back. The text is `RestrictedDerived`: it may still reveal protected facts
/// and is never authoritative business state.
#[derive(Debug, Clone)]
pub struct RehydratedResponse {
    text: TrustedValue,
    /// Placeholders the model actually used and we substituted.
    pub substituted: Vec<String>,
    /// True when the model echoed a label this job never issued. The response
    /// is rejected in that case; the flag exists for the honest error.
    pub had_unknown_placeholder: bool,
}

impl RehydratedResponse {
    /// The rehydrated text as an untrusted derived value. The caller decides
    /// whether to show it; nothing here makes it authoritative.
    pub fn text(&self) -> &TrustedValue {
        &self.text
    }
}

/// Validate a provider response against the job that produced it, then put
/// the withheld values back.
///
/// Rejects an expired job, an empty or oversized response, control characters,
/// and any bracketed label this job did not issue — a model inventing
/// `[ORG_7]` is either confused or being steered, and either way the result is
/// not shown.
pub fn accept(
    job: &PublicJob,
    response: &str,
    now_unix: i64,
) -> Result<RehydratedResponse, ResponseError> {
    if !job.is_live_at(now_unix) {
        return Err(ResponseError::JobExpired);
    }
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return Err(ResponseError::Empty);
    }
    let length = trimmed.chars().count();
    if length > MAX_RESPONSE_CHARS {
        return Err(ResponseError::TooLarge {
            actual: length,
            maximum: MAX_RESPONSE_CHARS,
        });
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t' && ch != '\r')
    {
        return Err(ResponseError::ControlCharacters);
    }

    let known = job.placeholders();
    for token in bracketed_tokens(trimmed) {
        if !known.contains_key(&token) {
            return Err(ResponseError::UnknownPlaceholder(token));
        }
    }

    let mut text = trimmed.to_owned();
    let mut substituted = Vec::new();
    for (token, value) in known {
        if text.contains(token.as_str()) {
            text = text.replace(token.as_str(), value);
            substituted.push(token.clone());
        }
    }
    Ok(RehydratedResponse {
        text: TrustedValue::restricted_derived(text),
        substituted,
        had_unknown_placeholder: false,
    })
}

/// Every `[LABEL_N]`-shaped token in the text. Deliberately narrow: only the
/// shape the compiler issues counts, so ordinary brackets in prose are not
/// treated as placeholders.
fn bracketed_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'[' {
            index += 1;
            continue;
        }
        let Some(close) = text[index..].find(']') else {
            break;
        };
        let token = &text[index..index + close + 1];
        let inner = &token[1..token.len() - 1];
        let shaped = inner.rsplit_once('_').is_some_and(|(label, number)| {
            !label.is_empty()
                && label.bytes().all(|byte| byte.is_ascii_uppercase())
                && !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
        });
        if shaped && !out.contains(&token.to_owned()) {
            out.push(token.to_owned());
        }
        index += close + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_compiler_shaped_labels_count_as_placeholders() {
        assert_eq!(
            bracketed_tokens("hello [ORG_1] and [PERSON_2] again [ORG_1]"),
            ["[ORG_1]", "[PERSON_2]"]
        );
        assert!(bracketed_tokens("a [note] and [lower_1] and [ORG_] and [_1]").is_empty());
        assert!(bracketed_tokens("no brackets at all").is_empty());
        assert!(bracketed_tokens("unclosed [ORG_1").is_empty());
    }
}
