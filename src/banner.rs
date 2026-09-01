//! Renders short labels - the SDLC phase name - as large ASCII "word art" so
//! the current phase is impossible to miss in a long console log.
//!
//! The font is a 5-row block font covering the characters phase names use
//! (letters, digits, `-`, `_`, `.` and space). Anything else renders as a
//! filled box rather than disappearing.

/// Height of every glyph, in console rows.
pub const ROWS: usize = 5;

/// Blank column between adjacent glyphs.
const GAP: &str = " ";

/// Renders `text` as `ROWS` lines of block characters.
///
/// Lines have no trailing blanks, so callers can indent them freely.
pub fn big_text(text: &str) -> Vec<String> {
    let glyphs: Vec<[&str; ROWS]> = text.chars().map(glyph).collect();

    (0..ROWS)
        .map(|row| {
            let line: Vec<&str> = glyphs.iter().map(|g| g[row]).collect();
            line.join(GAP).trim_end().replace('X', "█")
        })
        .collect()
}

/// The 5-row pattern for one character. `X` is ink, a space is background;
/// glyphs are naturally proportional, so widths vary between characters.
fn glyph(c: char) -> [&'static str; ROWS] {
    match c.to_ascii_lowercase() {
        ' ' => ["  ", "  ", "  ", "  ", "  "],
        '-' => ["   ", "   ", "XXX", "   ", "   "],
        '_' => ["    ", "    ", "    ", "    ", "XXXX"],
        '.' => [" ", " ", " ", " ", "X"],
        'a' => [" XX ", "X  X", "XXXX", "X  X", "X  X"],
        'b' => ["XXX ", "X  X", "XXX ", "X  X", "XXX "],
        'c' => [" XXX", "X   ", "X   ", "X   ", " XXX"],
        'd' => ["XXX ", "X  X", "X  X", "X  X", "XXX "],
        'e' => ["XXXX", "X   ", "XXX ", "X   ", "XXXX"],
        'f' => ["XXXX", "X   ", "XXX ", "X   ", "X   "],
        'g' => [" XXX", "X   ", "X XX", "X  X", " XXX"],
        'h' => ["X  X", "X  X", "XXXX", "X  X", "X  X"],
        'i' => ["XXX", " X ", " X ", " X ", "XXX"],
        'j' => ["  XX", "   X", "   X", "X  X", " XX "],
        'k' => ["X  X", "X X ", "XX  ", "X X ", "X  X"],
        'l' => ["X   ", "X   ", "X   ", "X   ", "XXXX"],
        'm' => ["X   X", "XX XX", "X X X", "X   X", "X   X"],
        'n' => ["X   X", "XX  X", "X X X", "X  XX", "X   X"],
        'o' => [" XX ", "X  X", "X  X", "X  X", " XX "],
        'p' => ["XXX ", "X  X", "XXX ", "X   ", "X   "],
        'q' => [" XX ", "X  X", "X  X", "X XX", " XXX"],
        'r' => ["XXX ", "X  X", "XXX ", "X X ", "X  X"],
        's' => [" XXX", "X   ", " XX ", "   X", "XXX "],
        't' => ["XXX", " X ", " X ", " X ", " X "],
        'u' => ["X  X", "X  X", "X  X", "X  X", " XX "],
        'v' => ["X   X", "X   X", "X   X", " X X ", "  X  "],
        'w' => ["X   X", "X   X", "X X X", "XX XX", "X   X"],
        'x' => ["X   X", " X X ", "  X  ", " X X ", "X   X"],
        'y' => ["X   X", " X X ", "  X  ", "  X  ", "  X  "],
        'z' => ["XXXX", "   X", "  X ", " X  ", "XXXX"],
        '0' => [" XX ", "X  X", "X  X", "X  X", " XX "],
        '1' => [" X ", "XX ", " X ", " X ", "XXX"],
        '2' => ["XXX ", "   X", " XX ", "X   ", "XXXX"],
        '3' => ["XXX ", "   X", " XX ", "   X", "XXX "],
        '4' => ["X  X", "X  X", "XXXX", "   X", "   X"],
        '5' => ["XXXX", "X   ", "XXX ", "   X", "XXX "],
        '6' => [" XXX", "X   ", "XXX ", "X  X", " XX "],
        '7' => ["XXXX", "   X", "  X ", " X  ", " X  "],
        '8' => [" XX ", "X  X", " XX ", "X  X", " XX "],
        '9' => [" XX ", "X  X", " XXX", "   X", "XXX "],
        _ => ["XXXX", "X  X", "X  X", "X  X", "XXXX"],
    }
}

#[cfg(test)]
#[path = "banner_tests.rs"]
mod banner_tests;
