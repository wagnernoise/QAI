// ── Simple text input with cursor ───────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize, // byte position
}

impl TextInput {
    pub fn new() -> Self { Self::default() }

    pub fn lines(&self) -> Vec<String> {
        self.value.lines().map(|l| l.to_string()).collect()
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor == 0 { return; }
        let prev = self.value[..self.cursor]
            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
        self.value.remove(prev);
        self.cursor = prev;
    }

    pub fn delete_char_after(&mut self) {
        if self.cursor >= self.value.len() { return; }
        self.value.remove(self.cursor);
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 { return; }
        self.cursor = self.value[..self.cursor]
            .char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.value.len() { return; }
        let ch = self.value[self.cursor..].chars().next().unwrap();
        self.cursor += ch.len_utf8();
    }

    pub fn move_home(&mut self) { self.cursor = 0; }
    pub fn move_end(&mut self) { self.cursor = self.value.len(); }

    /// Move cursor up one visual row (accounting for word-wrap and newlines).
    pub fn move_up(&mut self, inner_width: usize) {
        if inner_width == 0 { return; }
        let before = &self.value[..self.cursor];
        // Collect all chars before cursor with their byte positions
        let chars: Vec<(usize, char)> = before.char_indices().collect();
        if chars.is_empty() { return; }
        // Find the column of the cursor on its current visual row
        let col = self.cursor_col(inner_width);
        let cur_row = self.cursor_row(inner_width) as usize;
        if cur_row == 0 { return; }
        let target_row = cur_row - 1;
        // Walk all chars in the full value to find the position at (target_row, col)
        self.cursor = self.pos_at_row_col(inner_width, target_row, col);
    }

    /// Move cursor down one visual row.
    pub fn move_down(&mut self, inner_width: usize) {
        if inner_width == 0 { return; }
        let cur_row = self.cursor_row(inner_width) as usize;
        let col = self.cursor_col(inner_width);
        let target_row = cur_row + 1;
        let new_pos = self.pos_at_row_col(inner_width, target_row, col);
        // Only move if we actually advanced
        if new_pos > self.cursor || target_row == 0 {
            self.cursor = new_pos;
        }
    }

    /// Returns the visual column of the cursor within its current visual row.
    pub fn cursor_col(&self, inner_width: usize) -> usize {
        if inner_width == 0 { return 0; }
        let before = &self.value[..self.cursor];
        // Find the last newline before cursor
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_chars = before[line_start..].chars().count();
        line_chars % inner_width.max(1)
    }

    /// Returns the byte position in `self.value` corresponding to visual (row, col).
    fn pos_at_row_col(&self, inner_width: usize, target_row: usize, target_col: usize) -> usize {
        if inner_width == 0 { return self.cursor; }
        let mut row = 0usize;
        let mut col = 0usize;
        let mut best = self.cursor;
        let mut found = false;
        for (byte_pos, ch) in self.value.char_indices() {
            if row == target_row && (col >= target_col || ch == '\n') {
                best = byte_pos;
                found = true;
                break;
            }
            if ch == '\n' {
                if row == target_row { best = byte_pos; found = true; break; }
                row += 1;
                col = 0;
            } else {
                col += 1;
                if inner_width > 0 && col >= inner_width {
                    if row == target_row { best = byte_pos + ch.len_utf8(); found = true; break; }
                    row += 1;
                    col = 0;
                }
            }
        }
        if !found && row == target_row {
            best = self.value.len();
        }
        best
    }

    pub fn clear(&mut self) { self.value.clear(); self.cursor = 0; }

    /// Returns the wrapped row index of the cursor given an available inner width.
    /// Used to scroll the input box so the cursor is always visible.
    pub fn cursor_row(&self, inner_width: usize) -> u16 {
        if inner_width == 0 { return 0; }
        let before = &self.value[..self.cursor];
        let mut row: usize = 0;
        let logical_lines: Vec<&str> = before.split('\n').collect();
        let n = logical_lines.len();
        for (i, logical_line) in logical_lines.iter().enumerate() {
            let char_count = logical_line.chars().count();
            if i + 1 < n {
                // Not the last segment: this logical line plus its newline occupies
                // at least 1 row (empty line) or ceil(chars/width) rows.
                row += if char_count == 0 { 1 } else { char_count.div_ceil(inner_width) };
            } else {
                // Last segment: cursor sits at position char_count within this line.
                // Row within this line = char_count / inner_width (integer division).
                row += char_count / inner_width;
            }
        }
        row as u16
    }

    /// Returns (text_before_cursor, cursor_char_or_space, text_after_cursor)
    pub fn split_at_cursor(&self) -> (&str, &str, &str) {
        let before = &self.value[..self.cursor];
        if self.cursor >= self.value.len() {
            (before, " ", "")
        } else {
            let ch_end = self.cursor + self.value[self.cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            (before, &self.value[self.cursor..ch_end], &self.value[ch_end..])
        }
    }
}


// ── TextInput key handler ─────────────────────────────────────────────────────

pub fn handle_text_input_key(input: &mut TextInput, key: crossterm::event::KeyEvent, inner_width: usize) {
    use crossterm::event::KeyCode;
    use crossterm::event::KeyModifiers;
    match key.code {
        KeyCode::Char(c) => input.insert_char(c),
        KeyCode::Backspace => input.delete_char_before(),
        KeyCode::Delete => input.delete_char_after(),
        KeyCode::Left => input.move_left(),
        KeyCode::Right => input.move_right(),
        KeyCode::Home => input.move_home(),
        KeyCode::End => input.move_end(),
        KeyCode::Up => input.move_up(inner_width),
        KeyCode::Down => input.move_down(inner_width),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => input.insert_newline(),
        _ => {}
    }
}

