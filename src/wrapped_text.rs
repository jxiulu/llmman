use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub struct WrappedText<'a> {
    original: &'a str,
    lines: Vec<&'a str>,
    wrap_width: usize,
}

pub trait WrapText<'a> {
    fn wrapped_lines(&'a self, width: usize) -> Vec<&'a str>;
}

impl<'a> WrapText<'a> for str {
    fn wrapped_lines(&'a self, width: usize) -> Vec<&'a str> {
        wrap_text(self, width)
    }
}

impl<'a> WrapText<'a> for String {
    fn wrapped_lines(&'a self, width: usize) -> Vec<&'a str> {
        wrap_text(self, width)
    }
}

impl<'a> From<WrappedText<'a>> for &'a str {
    fn from(value: WrappedText<'a>) -> Self {
        value.original
    }
}

impl<'a> From<&WrappedText<'a>> for &'a str {
    fn from(value: &WrappedText<'a>) -> Self {
        value.original
    }
}

impl<'a> Display for WrappedText<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}

impl<'a> WrappedText<'a> {
    pub fn new(t: &'a str, w: usize) -> Self {
        Self {
            original: t,
            lines: wrap_text(t, w),
            wrap_width: w,
        }
    }

    pub fn original(&self) -> &'a str {
        self.original
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    pub fn width(&self) -> usize {
        self.wrap_width
    }

    pub fn lines(&self) -> &[&'a str] {
        &self.lines
    }

    pub fn rewrap(&self, w: usize) -> Self {
        Self::new(self.original, w)
    }
}


// tracks the current output row as a byte range [row_start, row_end) within
// the current `line` slice, extending it word by word without any allocations.
pub fn wrap_text<'a>(text: &'a str, width: usize) -> Vec<&'a str> {
    if width == 0 {
        return vec![text];
    }

    let mut rows: Vec<&'a str> = Vec::new();

    for line in text.split('\n') {
        if line.is_empty() {
            rows.push(line);
            continue;
        }

        // word.as_ptr() - line_base gives the byte offset of any word within this line.
        let line_base = line.as_ptr() as usize;
        let mut row_start: usize = 0;
        let mut row_end: usize = 0;
        let mut row_char_count: usize = 0;
        let mut row_started = false;

        for mut word in line.split(' ') {
            // Words longer than width are split aggressively at exactly `width` chars.
            // Each head piece is emitted immediately; the leftover tail continues below.
            while word.chars().count() > width {
                if row_started {
                    rows.push(&line[row_start..row_end]);
                    row_started = false;
                    row_char_count = 0;
                }

                let split_index = word
                    .char_indices()
                    .nth(width)
                    .map(|(i, _)| i)
                    .unwrap_or(word.len());

                let (head, tail) = word.split_at(split_index);
                rows.push(head);
                word = tail;
            }

            let word_char_count = word.chars().count();
            let extra = if row_started { 1 } else { 0 }; // +1 for the space separator

            // Flush the current row before appending if adding this word would overflow.
            if row_started && row_char_count + extra + word_char_count > width {
                rows.push(&line[row_start..row_end]);
                row_started = false;
                row_char_count = 0;
            }

            // Convert the word pointer back to a byte offset so we can slice `line`.
            // This works because split() yields subslices of the original string.
            let word_offset = word.as_ptr() as usize - line_base;
            if !row_started {
                row_start = word_offset;
                row_end = word_offset + word.len();
                row_char_count = word_char_count;
                row_started = true;
            } else {
                // Stretch the row's right boundary to include the space + this word.
                // The space is already in `line` between the previous word and this one.
                row_end = word_offset + word.len();
                row_char_count += 1 + word_char_count;
            }
        }

        if row_started {
            rows.push(&line[row_start..row_end]);
        }
    }

    rows
}
