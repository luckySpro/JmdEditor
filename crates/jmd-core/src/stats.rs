//! Word counts for the status bar. CJK characters count as one word each.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub words: usize,
    pub characters: usize,
    /// Minutes at 220 Latin words or 400 CJK characters a minute, at least 1.
    pub reading_minutes: usize,
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xAC00..=0xD7AF)
}

pub fn stats(text: &str) -> Stats {
    let mut latin_words = 0;
    let mut cjk = 0;
    let mut in_word = false;
    let mut characters = 0;
    for c in text.chars() {
        if !c.is_whitespace() {
            characters += 1;
        }
        if is_cjk(c) {
            cjk += 1;
            in_word = false;
        } else if c.is_alphanumeric() {
            if !in_word {
                latin_words += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    let minutes = (latin_words as f64 / 220.0 + cjk as f64 / 400.0).ceil() as usize;
    Stats {
        words: latin_words + cjk,
        characters,
        reading_minutes: minutes.max(1),
    }
}
