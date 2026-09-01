use super::*;

/// Every character the font is expected to cover.
fn supported() -> Vec<char> {
    "abcdefghijklmnopqrstuvwxyz0123456789 -_."
        .chars()
        .collect()
}

#[test]
fn every_glyph_is_a_rectangle_of_the_same_height() {
    for c in supported() {
        let rows = glyph(c);
        let width = rows[0].chars().count();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(
                row.chars().count(),
                width,
                "glyph {:?} row {} is {:?}, which is not {} columns wide",
                c,
                i,
                row,
                width
            );
        }
        assert!(width > 0, "glyph {:?} has no width", c);
    }
}

#[test]
fn renders_letters_side_by_side_in_block_characters() {
    assert_eq!(
        big_text("hi"),
        vec![
            "█  █ ███",
            "█  █  █",
            "████  █",
            "█  █  █",
            "█  █ ███",
        ]
    );
}

#[test]
fn renders_a_hyphen_as_a_bar_on_the_middle_row_only() {
    assert_eq!(big_text("-"), vec!["", "", "███", "", ""]);
}

#[test]
fn is_case_insensitive() {
    assert_eq!(big_text("AI"), big_text("ai"));
}

#[test]
fn renders_an_unsupported_character_as_a_box_rather_than_dropping_it() {
    let rendered = big_text("&");
    assert_eq!(rendered[0], "████");
    assert_eq!(rendered[2], "█  █");
}

#[test]
fn produces_one_line_per_row_with_no_trailing_blanks() {
    let rendered = big_text("ai-design");
    assert_eq!(rendered.len(), ROWS);
    for line in &rendered {
        assert_eq!(line.trim_end(), line, "line has trailing blanks: {:?}", line);
    }
}

#[test]
fn the_longest_phase_name_still_fits_a_standard_terminal() {
    // Phase banners are printed with a two-space indent, so 80 columns is the budget.
    for phase in [
        "ai-requirements",
        "ai-design",
        "ai-development",
        "ai-review",
        "ai-pr-creation",
        "ai-done",
    ] {
        let widest = big_text(phase).iter().map(|l| l.chars().count()).max().unwrap();
        assert!(widest + 2 <= 80, "{} renders {} columns wide", phase, widest);
    }
}
