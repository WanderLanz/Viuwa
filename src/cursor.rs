//! ASCII cursor functions and structs, by default words are seperated by is_ascii_whitespace.
// HELP: I'm sure there's a crate for this I just can't find it.
use ::core::iter::zip;
use viuwa_ansi::consts::*;

use super::*;

/// ASCII cursor functions, by default words are seperated by is_ascii_whitespace.
pub mod ascii {
    use super::*;
    /// first end index of a 'word' segment (including whitespace) to the right of cur in [cur + 1..]
    // TODO: move some of the agnostic functions of AsciiPrompt and AsciiCursor to the this module
    #[inline]
    pub fn segment_end(buf: &[u8], cur: usize) -> usize {
        debug_assert!(cur <= buf.len());
        if let Some(mid) = buf.get(cur) {
            if let Some(right) = buf.get(cur + 1..) {
                let cond = mid.is_ascii_whitespace();
                for (i, b) in zip(cur + 1.., right) {
                    if (cond ^ b.is_ascii_whitespace()) as u8 != 0 {
                        return i;
                    }
                }
            }
        }
        buf.len()
    }
    /// first start index of a 'word' segment (including whitespace) to the left of cur in [..=cur]
    #[inline]
    pub fn segment_start(buf: &[u8], cur: usize) -> usize {
        debug_assert!(cur <= buf.len());
        if let Some(mid) = buf.get(cur) {
            if let Some(left) = buf.get(..cur) {
                let cond = mid.is_ascii_whitespace();
                for (i, b) in left.iter().enumerate().rev() {
                    if (cond ^ b.is_ascii_whitespace()) as u8 != 0 {
                        return i + 1;
                    }
                }
            }
            0
        } else {
            buf.len()
        }
    }
    /// first start index of a 'word' segment (including whitespace) to the left of cur in [start..=cur]
    #[inline]
    pub fn segment_start_from(buf: &[u8], start: usize, cur: usize) -> usize {
        debug_assert!(start <= cur && cur <= buf.len());
        if let Some(mid) = buf.get(cur) {
            if let Some(left) = buf.get(start..cur) {
                let cond = mid.is_ascii_whitespace();
                for (i, b) in left.iter().enumerate().rev() {
                    if (cond ^ b.is_ascii_whitespace()) as u8 != 0 {
                        return i + start + 1;
                    }
                }
            }
            start
        } else {
            buf.len()
        }
    }
    /// first end index of a non-whitespace word to the right of cur in [cur + 1..]
    #[inline]
    pub fn word_end(buf: &[u8], cur: usize) -> usize {
        debug_assert!(cur <= buf.len());
        if let Some(mid) = buf.get(cur) {
            let mut last = mid.is_ascii_whitespace() as u8;
            if let Some(right) = buf.get(cur + 1..) {
                for (i, b) in zip(cur + 1.., right) {
                    let new = b.is_ascii_whitespace() as u8;
                    // if we transition from non-whitespace to whitespace, we've found the end of the word
                    if new.saturating_sub(last) == 1 {
                        return i;
                    }
                    last = new;
                }
            }
        }
        buf.len()
    }
    /// first start index of a non-whitespace word to the left of cur in [..=cur]
    #[inline]
    pub fn word_start(buf: &[u8], cur: usize) -> usize {
        debug_assert!(cur <= buf.len());
        // we count end as whitespace
        let mut last = if let Some(mid) = buf.get(cur) { mid.is_ascii_whitespace() as u8 } else { 1 };
        if let Some(left) = buf.get(..cur) {
            for (i, b) in left.iter().enumerate().rev() {
                let new = b.is_ascii_whitespace() as u8;
                // if we transition from non-whitespace to whitespace, we've found the start of the word
                if new.saturating_sub(last) == 1 {
                    return i + 1;
                }
                last = new;
            }
        }
        0
    }
    /// first start index of a non-whitespace word to the left of cur in [start..=cur]
    #[inline]
    pub fn word_start_from(buf: &[u8], start: usize, cur: usize) -> usize {
        debug_assert!(start <= cur && cur <= buf.len());
        // we count end as whitespace
        let mut last = if let Some(mid) = buf.get(cur) { mid.is_ascii_whitespace() as u8 } else { 1 };
        if let Some(left) = buf.get(start..cur) {
            for (i, b) in left.iter().enumerate().rev() {
                let new = b.is_ascii_whitespace() as u8;
                // if we transition from non-whitespace to whitespace, we've found the start of the word
                if new.saturating_sub(last) == 1 {
                    return i + start + 1;
                }
                last = new;
            }
        }
        start
    }
}

/// Terminal cursor for ASCII string prompts, with a left bound on the cursor allowing for prompts in buffer.
/// Does not flush the terminal.
// MAYBE: utf8 version of this? I'm sure there's a crate for this I just don't know it
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsciiPrompt {
    buf: String,
    cur: u16,
    start: u16,
}
impl AsciiPrompt {
    #[inline]
    pub fn new(buf: String, cur: u16, start: u16) -> Option<Self> {
        assert!(start <= cur && (cur as usize) <= buf.len());
        if buf.is_ascii() {
            Some(Self { buf, cur, start })
        } else {
            None
        }
    }
    #[inline]
    pub unsafe fn new_unchecked(buf: String, cur: u16, start: u16) -> Self { Self { buf, cur, start } }
    #[inline(always)]
    pub fn buf(&self) -> &str { &self.buf }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
    #[inline(always)]
    pub fn bytes(&self) -> &[u8] { self.buf.as_bytes() }
    #[inline(always)]
    pub fn into_inner(self) -> (String, u16, u16) { (self.buf, self.cur, self.start) }
    #[inline(always)]
    pub fn len(&self) -> usize { self.buf.len() }
    #[inline(always)]
    pub fn cur(&self) -> u16 { self.cur }
    #[inline(always)]
    pub fn start(&self) -> u16 { self.start }
    #[inline(always)]
    pub fn at_end(&self) -> bool { (self.cur as usize) == self.len() }
    #[inline(always)]
    pub fn at_last(&self) -> bool { (self.cur as usize) + 1 == self.len() }
    #[inline(always)]
    pub fn at_start(&self) -> bool { self.cur == self.start }
    #[inline(always)]
    pub fn idx(&self) -> usize { self.cur as usize }
    /// Update the terminal cursor position.
    #[inline(always)]
    fn reposition(&mut self, term: &mut impl Terminal) {
        _execute!(term, cursor_to_col(self.cur as u16));
    }
    #[inline]
    pub fn right(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur += 1;
            _execute!(term, write_as(CURSOR_RIGHT));
        }
    }
    #[inline]
    pub fn left(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            _execute!(term, write_as(CURSOR_LEFT));
        }
    }
    #[inline]
    pub fn to_start(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur = self.start;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to_end(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur = self.len() as u16;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to_last(&mut self, term: &mut impl Terminal) {
        if !self.at_last() {
            self.cur = (self.len() - 1) as u16;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to(&mut self, term: &mut impl Terminal, idx: usize) {
        if idx != self.idx() && (self.start as usize..=self.len()).contains(&idx) {
            self.cur = idx as u16;
            self.reposition(term);
        }
    }
    /// Delete the character at the cursor position.
    #[inline]
    pub fn delete(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            if self.at_last() {
                self.buf.pop();
                _execute!(term, write_as(CURSOR_LEFT), clear_line_to_end());
            } else {
                self.buf.remove(self.idx());
                _execute!(
                    term,
                    write_as(CURSOR_LEFT),
                    clear_line_to_end(),
                    write_as(&self.buf[self.idx()..]),
                    cursor_to_col(self.cur)
                );
            }
        }
    }
    /// Insert a character at the cursor position.
    #[inline]
    pub fn insert(&mut self, term: &mut impl Terminal, c: char) {
        if c.is_ascii() {
            if self.at_end() {
                self.buf.push(c);
            } else {
                self.buf.insert(self.idx(), c);
            }
            _execute!(term, write_as(&self.buf[self.idx()..]));
            self.cur += 1;
            _execute!(term, cursor_to_col(self.cur));
        }
    }
    /// Returns the end index of the current word segment (exclusive), including end of string.
    /// Note that this considers whitespace to be a word segment.
    #[inline]
    pub fn segment_end(&self) -> usize { ascii::segment_end(self.bytes(), self.idx()) }
    /// Returns the start index of the current word segment (inclusive), including start of string.
    /// Note that this considers whitespace to be a word segment.
    #[inline]
    pub fn segment_start(&self) -> usize { ascii::segment_start_from(self.bytes(), self.start as usize, self.idx()) }
    /// Get the first start index of a non-whitespace word segment to the left of the cursor, may be the cursor index.
    #[inline]
    pub fn word_start(&self) -> usize { ascii::word_start_from(self.bytes(), self.start as usize, self.idx()) }
    #[inline]
    pub fn word_end(&mut self) -> usize { ascii::word_end(self.bytes(), self.idx()) }
    /// Move the cursor to the start of the word that the cursor is currently on or the previous word if the cursor is already at the start of a word.
    /// This corresponds to the `Ctrl + Left` keybinding on most terminals and text editors.
    #[inline]
    pub fn left_word(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            self.cur = self.word_start() as u16;
            self.reposition(term);
        }
    }
    /// Move the cursor to the end of the word that the cursor is currently on or the next word if the cursor is already at the end of a word.
    /// This corresponds to the `Ctrl + Right` keybinding on most terminals and text editors.
    #[inline]
    pub fn right_word(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur = self.word_end() as u16;
            self.reposition(term);
        }
    }
    /// Delete the word that the cursor is currently on, from the cursor to the start of the word.
    #[inline]
    pub fn delete_word(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            let orig = self.cur;
            self.cur -= 1;
            self.cur = self.word_start() as u16;
            let _ = self.buf.drain(self.idx()..orig as usize);
            if !self.at_end() {
                _execute!(
                    term,
                    cursor_to_col(self.cur),
                    clear_line_to_end(),
                    write_as(&self.buf[self.idx()..]),
                    cursor_to_col(self.cur)
                );
            } else {
                _execute!(term, cursor_to_col(self.cur), clear_line_to_end());
            }
        }
    }
    /// Get the word that the cursor is currently on.
    /// Returns None if there is no word.
    #[inline]
    pub fn get_word(&self) -> Option<&str> {
        if self.bytes().get(self.idx())?.is_ascii_whitespace() {
            // we are at end of buf or whitespace
            None
        } else {
            let start = self.segment_start();
            if !self.bytes()[start].is_ascii_whitespace() {
                Some(&self.buf[start..self.segment_end()])
            } else {
                None
            }
        }
    }
    /// Get the word that the cursor is currently on or the next word.
    /// Returns None if there is no words left.
    #[inline]
    pub fn next_word(&self) -> Option<&str> {
        let start =
            if self.bytes().get(self.idx())?.is_ascii_whitespace() { self.segment_end() } else { self.segment_start() };
        if start == self.len() {
            None
        } else {
            Some(&self.buf[start..ascii::segment_end(self.bytes(), start)])
        }
    }
    /// Get the word that the cursor is currently on or the previous word.
    /// Returns None if there is no words left.
    #[inline]
    pub fn prev_word(&self) -> Option<&str> {
        let end =
            if self.bytes().get(self.idx())?.is_ascii_whitespace() { self.segment_start() } else { self.segment_end() };
        if end == self.start as usize {
            None
        } else {
            Some(&self.buf[ascii::segment_start_from(self.bytes(), self.start as usize, end)..end])
        }
    }
}

/// Terminal cursor for an ASCII line.
/// Does not flush the terminal.
// MAYBE: utf8 version of this? I'm sure there's a crate for this I just don't know it
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsciiCursor {
    buf: String,
    cur: u16,
}
impl AsciiCursor {
    #[inline]
    pub fn new(buf: String, cur: u16) -> Option<Self> {
        assert!((cur as usize) <= buf.len());
        if buf.is_ascii() {
            Some(Self { buf, cur })
        } else {
            None
        }
    }
    #[inline]
    pub unsafe fn new_unchecked(buf: String, cur: u16) -> Self { Self { buf, cur } }
    #[inline(always)]
    pub fn buf(&self) -> &str { &self.buf }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
    #[inline(always)]
    pub fn bytes(&self) -> &[u8] { self.buf.as_bytes() }
    #[inline(always)]
    pub fn into_inner(self) -> (String, u16) { (self.buf, self.cur) }
    #[inline(always)]
    pub fn len(&self) -> usize { self.buf.len() }
    #[inline(always)]
    pub fn cur(&self) -> u16 { self.cur }
    #[inline(always)]
    pub fn at_end(&self) -> bool { (self.cur as usize) == self.len() }
    #[inline(always)]
    pub fn at_last(&self) -> bool { (self.cur as usize) + 1 == self.len() }
    #[inline(always)]
    pub fn at_start(&self) -> bool { self.cur == 0 }
    #[inline(always)]
    pub fn idx(&self) -> usize { self.cur as usize }
    /// Update the terminal cursor position.
    #[inline(always)]
    fn reposition(&mut self, term: &mut impl Terminal) {
        _execute!(term, cursor_to_col(self.cur as u16));
    }
    #[inline]
    pub fn right(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur += 1;
            _execute!(term, write_as(CURSOR_RIGHT));
        }
    }
    #[inline]
    pub fn left(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            _execute!(term, write_as(CURSOR_LEFT));
        }
    }
    #[inline]
    pub fn to_start(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur = 0;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to_end(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur = self.len() as u16;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to_last(&mut self, term: &mut impl Terminal) {
        if !self.at_last() {
            self.cur = (self.len() - 1) as u16;
            self.reposition(term);
        }
    }
    #[inline]
    pub fn to(&mut self, term: &mut impl Terminal, idx: usize) {
        if idx != self.idx() && idx <= self.len() {
            self.cur = idx as u16;
            self.reposition(term);
        }
    }
    /// Delete the character at the cursor position.
    #[inline]
    pub fn delete(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            if self.at_last() {
                self.buf.pop();
                _execute!(term, write_as(CURSOR_LEFT), clear_line_to_end());
            } else {
                self.buf.remove(self.idx());
                _execute!(
                    term,
                    write_as(CURSOR_LEFT),
                    clear_line_to_end(),
                    write_as(&self.buf[self.idx()..]),
                    cursor_to_col(self.cur)
                );
            }
        }
    }
    /// Insert a character at the cursor position.
    #[inline]
    pub fn insert(&mut self, term: &mut impl Terminal, c: char) {
        if c.is_ascii() {
            if self.at_end() {
                self.buf.push(c);
            } else {
                self.buf.insert(self.idx(), c);
            }
            _execute!(term, write_as(&self.buf[self.idx()..]));
            self.cur += 1;
            _execute!(term, cursor_to_col(self.cur));
        }
    }
    /// Returns the end index of the current word segment (exclusive), including end of string.
    /// Note that this considers whitespace to be a word segment.
    #[inline]
    pub fn segment_end(&self) -> usize { ascii::segment_end(self.bytes(), self.idx()) }
    /// Returns the start index of the current word segment (inclusive), including start of string.
    /// Note that this considers whitespace to be a word segment.
    #[inline]
    pub fn segment_start(&self) -> usize { ascii::segment_start(self.bytes(), self.idx()) }
    /// Get the first start index of a non-whitespace word segment to the left of the cursor, may be the cursor index.
    #[inline]
    pub fn word_start(&self) -> usize { ascii::word_start(self.bytes(), self.idx()) }
    #[inline]
    pub fn word_end(&mut self) -> usize { ascii::word_end(self.bytes(), self.idx()) }
    /// Move the cursor to the start of the word that the cursor is currently on or the previous word if the cursor is already at the start of a word.
    /// This corresponds to the `Ctrl + Left` keybinding on most terminals and text editors.
    #[inline]
    pub fn left_word(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            self.cur -= 1;
            self.cur = self.word_start() as u16;
            self.reposition(term);
        }
    }
    /// Move the cursor to the end of the word that the cursor is currently on or the next word if the cursor is already at the end of a word.
    /// This corresponds to the `Ctrl + Right` keybinding on most terminals and text editors.
    #[inline]
    pub fn right_word(&mut self, term: &mut impl Terminal) {
        if !self.at_end() {
            self.cur = self.word_end() as u16;
            self.reposition(term);
        }
    }
    /// Delete the word that the cursor is currently on, from the cursor to the start of the word.
    #[inline]
    pub fn delete_word(&mut self, term: &mut impl Terminal) {
        if !self.at_start() {
            let orig = self.cur;
            self.cur -= 1;
            self.cur = self.word_start() as u16;
            let _ = self.buf.drain(self.idx()..orig as usize);
            if !self.at_end() {
                _execute!(
                    term,
                    cursor_to_col(self.cur),
                    clear_line_to_end(),
                    write_as(&self.buf[self.idx()..]),
                    cursor_to_col(self.cur)
                );
            } else {
                _execute!(term, cursor_to_col(self.cur), clear_line_to_end());
            }
        }
    }
    /// Get the word that the cursor is currently on.
    /// Returns None if there is no word.
    #[inline]
    pub fn get_word(&self) -> Option<&str> {
        if self.bytes().get(self.idx())?.is_ascii_whitespace() {
            // we are at end of buf or whitespace
            None
        } else {
            let start = self.segment_start();
            if !self.bytes()[start].is_ascii_whitespace() {
                Some(&self.buf[start..self.segment_end()])
            } else {
                None
            }
        }
    }
    /// Get the word that the cursor is currently on or the next word.
    /// Returns None if there is no words left.
    #[inline]
    pub fn next_word(&self) -> Option<&str> {
        let start =
            if self.bytes().get(self.idx())?.is_ascii_whitespace() { self.segment_end() } else { self.segment_start() };
        if start == self.len() {
            None
        } else {
            Some(&self.buf[start..ascii::segment_end(self.bytes(), start)])
        }
    }
    /// Get the word that the cursor is currently on or the previous word.
    /// Returns None if there is no words left.
    #[inline]
    pub fn prev_word(&self) -> Option<&str> {
        let end =
            if self.bytes().get(self.idx())?.is_ascii_whitespace() { self.segment_start() } else { self.segment_end() };
        if end == 0 {
            None
        } else {
            Some(&self.buf[ascii::segment_start(self.bytes(), end)..end])
        }
    }
}
