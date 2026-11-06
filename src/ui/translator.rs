use std::time::Duration;

const MAX_QUERY_BYTES: usize = 450;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub(super) fn translate(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("There is no message text to translate.".into());
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(5))
        .build();

    let mut remaining = text;
    let mut translated = String::with_capacity(text.len());

    while !remaining.is_empty() {
        let end = chunk_end(remaining);
        let chunk = &remaining[..end];
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("q", chunk);
        query.append_pair("langpair", "autodetect|pt-BR");
        let query = query.finish();
        let address = format!("https://api.mymemory.translated.net/get?{query}");
        let response = agent
            .get(&address)
            .call()
            .map_err(|error| format!("MyMemory request failed: {error}"))?;
        let body = response
            .into_string()
            .map_err(|error| format!("Could not read the translation response: {error}"))?;
        if body.len() > MAX_RESPONSE_BYTES {
            return Err("The translation service returned an unexpectedly large response.".into());
        }

        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| format!("The translation service returned invalid data: {error}"))?;
        let status = value
            .get("responseStatus")
            .and_then(|status| status.as_i64().or_else(|| status.as_str()?.parse().ok()))
            .unwrap_or_default();
        if status != 200 {
            let detail = value
                .get("responseDetails")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("The service rejected the request.");
            return Err(format!("MyMemory: {detail}"));
        }

        let part = value
            .get("responseData")
            .and_then(|data| data.get("translatedText"))
            .and_then(serde_json::Value::as_str)
            .filter(|part| !part.trim().is_empty())
            .ok_or_else(|| "The service did not return a translation.".to_owned())?;

        if !translated.is_empty() {
            translated.push(' ');
        }
        translated.push_str(&decode_entities(part));
        remaining = &remaining[end..];
    }

    Ok(translated)
}

fn chunk_end(text: &str) -> usize {
    if text.len() <= MAX_QUERY_BYTES {
        return text.len();
    }

    let mut end = MAX_QUERY_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    if let Some((index, character)) = text[..end]
        .char_indices()
        .rev()
        .find(|(_, character)| character.is_whitespace())
    {
        let boundary = index + character.len_utf8();
        if boundary > 0 {
            return boundary;
        }
    }

    end
}

fn decode_entities(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_chunks_respect_utf8_boundaries() {
        let text = "á".repeat(300);
        let end = chunk_end(&text);
        assert!(text.is_char_boundary(end));
        assert!(end <= MAX_QUERY_BYTES);
        assert!(end > 0);
    }

    #[test]
    fn short_translation_text_stays_in_one_chunk() {
        let text = "Olá, mundo!";
        assert_eq!(chunk_end(text), text.len());
    }
}
