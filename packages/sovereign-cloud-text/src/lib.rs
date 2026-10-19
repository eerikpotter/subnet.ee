//! Deterministic, bounded derived text. No I/O, identity, networking or editor engine.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};
use unicode_normalization::UnicodeNormalization;
mod office;
mod pdf;
#[cfg(test)]
mod tests;
pub const EXTRACTOR_VERSION: u32 = 2;
pub const MAX_INPUT: usize = 16 * 1024 * 1024;
pub const MAX_TEXT: usize = 256 * 1024;
pub const MAX_SECTIONS: usize = 512;
pub const MAX_SECTION_BYTES: usize = 2048;
pub const MAX_TERMS: usize = 4096;
pub const MAX_XML: usize = 2 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
    pub location: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    Ready,
    Empty,
    Partial,
    Unsupported,
    TooLarge,
    Unreadable,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Extraction {
    pub outcome: Outcome,
    pub sections: Vec<Section>,
}
#[derive(Default)]
struct Output {
    sections: Vec<Section>,
    bytes: usize,
    partial: bool,
}
impl Output {
    fn push(&mut self, location: &str, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let mut rest = text;
        while !rest.is_empty() {
            if self.sections.len() >= MAX_SECTIONS || self.bytes >= MAX_TEXT {
                self.partial = true;
                return;
            }
            let mut end = rest.len().min(MAX_SECTION_BYTES).min(MAX_TEXT - self.bytes);
            while end > 0 && !rest.is_char_boundary(end) {
                end -= 1;
            }
            if end == 0 {
                self.partial = true;
                return;
            }
            // Prefer a word boundary, so adjacent chunks don't split ordinary tokens.
            if end < rest.len() {
                if let Some((i, _)) = rest[..end]
                    .char_indices()
                    .rev()
                    .find(|(i, c)| *i > end / 2 && c.is_whitespace())
                {
                    end = i;
                }
            }
            self.sections.push(Section {
                location: location.chars().take(160).collect(),
                text: rest[..end].into(),
            });
            self.bytes += end;
            rest = rest[end..].trim_start();
        }
    }
    fn full(&self) -> bool {
        self.partial || self.sections.len() >= MAX_SECTIONS || self.bytes >= MAX_TEXT
    }
}
pub fn tokens(text: &str) -> Vec<String> {
    text.nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .map(str::to_owned)
        .collect()
}
/// Posting values point to one representative section per token, not duplicated text.
pub fn terms(sections: &[Section]) -> (BTreeMap<String, u32>, bool) {
    let mut terms = BTreeMap::new();
    let mut partial = false;
    for (i, section) in sections.iter().enumerate() {
        for term in tokens(&section.text) {
            if terms.contains_key(&term) {
                continue;
            }
            if terms.len() == MAX_TERMS {
                partial = true;
                continue;
            }
            terms.insert(term, i as u32);
        }
    }
    (terms, partial)
}
pub fn extract(name: &str, media_type: &str, craft: Option<&str>, bytes: &[u8]) -> Extraction {
    if bytes.len() > MAX_INPUT {
        return Extraction {
            outcome: Outcome::TooLarge,
            sections: vec![],
        };
    }
    let mut ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if media_type.ends_with("wordprocessingml.document") {
        ext = "docx".into();
    } else if media_type.ends_with("spreadsheetml.sheet") {
        ext = "xlsx".into();
    } else if media_type.ends_with("presentationml.presentation") {
        ext = "pptx".into();
    }
    let mut output = Output::default();
    let result = if craft == Some("wordcraft.wcraft")
        || name.to_ascii_lowercase().ends_with(".wcraft.json")
    {
        wordcraft(bytes, &mut output)
    } else if media_type == "application/pdf" || ext == "pdf" {
        // PdfCraft stores ordinary PDF bytes with a Craft association.
        pdf::extract(bytes, &mut output)
    } else if craft.is_some() {
        return Extraction {
            outcome: Outcome::Unsupported,
            sections: vec![],
        };
    } else if ["docx", "xlsx", "pptx"].contains(&ext.as_str()) {
        office::extract(bytes, &ext, &mut output)
    } else if media_type.starts_with("text/")
        || ["txt", "md", "markdown", "csv", "tsv"].contains(&ext.as_str())
    {
        match std::str::from_utf8(bytes) {
            Ok(text) => {
                output.push("text", text);
                Ok(())
            }
            Err(_) => Err(()),
        }
    } else {
        return Extraction {
            outcome: Outcome::Unsupported,
            sections: vec![],
        };
    };
    let outcome = if result.is_err() {
        if output.sections.is_empty() {
            Outcome::Unreadable
        } else {
            Outcome::Partial
        }
    } else if output.partial {
        Outcome::Partial
    } else if output.sections.is_empty() {
        Outcome::Empty
    } else {
        Outcome::Ready
    };
    Extraction {
        outcome,
        sections: output.sections,
    }
}
fn wordcraft(bytes: &[u8], out: &mut Output) -> Result<(), ()> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    if value
        .get("wordcraftNativeVersion")
        .is_some_and(|v| v.as_u64() != Some(1))
    {
        return Err(());
    }
    let body = value["body"].as_array().ok_or(())?;
    fn blocks(
        values: &[serde_json::Value],
        out: &mut Output,
        depth: usize,
        location: &str,
    ) -> Result<(), ()> {
        if depth > 32 {
            return Err(());
        }
        for (i, block) in values.iter().enumerate() {
            if out.full() {
                out.partial = true;
                break;
            }
            match block["kind"].as_str() {
                Some("para") => out.push(
                    &format!("{location}/paragraph:{}", i + 1),
                    block["text"].as_str().ok_or(())?,
                ),
                Some("table") => {
                    for (r, row) in block["rows"].as_array().ok_or(())?.iter().enumerate() {
                        for (c, cell) in row["cells"].as_array().ok_or(())?.iter().enumerate() {
                            blocks(
                                cell["blocks"].as_array().ok_or(())?,
                                out,
                                depth + 1,
                                &format!("{location}/table:{}/row:{}/cell:{}", i + 1, r + 1, c + 1),
                            )?;
                        }
                    }
                }
                _ => return Err(()),
            }
        }
        Ok(())
    }
    blocks(body, out, 0, "body")
}
