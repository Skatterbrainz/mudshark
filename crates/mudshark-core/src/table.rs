//! Minimal fixed-width table rendering for human-readable output.

/// Per-column text alignment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// Render `headers` + `rows` as an auto-sized table with a dashed separator
/// beneath the header. `align` selects per-column alignment; any column without
/// an entry defaults to [`Align::Left`]. The result ends with a trailing newline.
pub fn render(headers: &[&str], rows: &[Vec<String>], align: &[Align]) -> String {
    let cols = headers.len();
    let mut widths = vec![0usize; cols];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.chars().count();
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }

    let mut out = String::new();
    let header_cells: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    push_row(&mut out, &header_cells, &widths, align);

    let separators: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    push_row(&mut out, &separators, &widths, align);

    for row in rows {
        push_row(&mut out, row, &widths, align);
    }
    out
}

fn push_row(out: &mut String, cells: &[String], widths: &[usize], align: &[Align]) {
    let mut line = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            line.push_str("  ");
        }
        let w = widths[i];
        match align.get(i).copied().unwrap_or(Align::Left) {
            Align::Left => line.push_str(&format!("{cell:<w$}")),
            Align::Right => line.push_str(&format!("{cell:>w$}")),
        }
    }
    // Trailing padding is noise; drop it.
    out.push_str(line.trim_end());
    out.push('\n');
}
