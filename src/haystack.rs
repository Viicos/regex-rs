use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// Haystacks of at least this many bytes are searched without holding the GIL.
///
/// Releasing it has a small fixed cost, only worth paying when the search itself might take a while.
const DETACH_THRESHOLD: usize = 4096;

/// Runs a search on a haystack of `len` bytes, releasing the GIL if the haystack is large.
pub(crate) fn search<T: Send>(py: Python<'_>, len: usize, f: impl FnOnce() -> T + Send) -> T {
    if len >= DETACH_THRESHOLD {
        py.detach(f)
    } else {
        f()
    }
}

/// A Python `str` being searched, along with its UTF-8 representation.
///
/// The `regex` crate works with UTF-8 byte offsets, while Python strings are indexed by code points.
/// Both are identical for ASCII strings, so conversions are only needed for non-ASCII ones.
pub(crate) struct Haystack<'a, 'py> {
    pub obj: &'a Bound<'py, PyString>,
    pub text: &'a str,
    pub ascii: bool,
    char_len: usize,
}

impl<'a, 'py> Haystack<'a, 'py> {
    pub fn new(obj: &'a Bound<'py, PyString>) -> PyResult<Self> {
        let text = obj.to_str()?;
        let char_len = obj.len()?;
        Ok(Self {
            obj,
            text,
            ascii: text.len() == char_len,
            char_len,
        })
    }

    /// Converts a code point offset given by the user into a byte offset.
    pub fn byte_offset(&self, start: usize) -> PyResult<usize> {
        if start > self.char_len {
            return Err(PyValueError::new_err(format!(
                "start ({start}) must not be greater than the length of the haystack ({})",
                self.char_len
            )));
        }
        if self.ascii {
            return Ok(start);
        }
        Ok(self
            .text
            .char_indices()
            .nth(start)
            .map_or(self.text.len(), |(i, _)| i))
    }

    /// A cursor positioned at the given byte offset (and its code point equivalent).
    pub fn cursor_at(&self, byte: usize, chars: usize) -> Cursor {
        Cursor {
            ascii: self.ascii,
            floor: (byte, chars),
            pos: (byte, chars),
        }
    }
}

/// A match position, both in UTF-8 bytes (for slicing the Rust string) and in code points.
#[derive(Clone, Copy)]
pub(crate) struct Span {
    pub byte_start: usize,
    pub byte_end: usize,
    pub start: usize,
    pub end: usize,
}

/// Converts byte offsets into code point offsets.
///
/// Converting an offset requires counting the code points before it. The cursor remembers the last
/// converted position, so that converting increasing offsets (as done when iterating over matches)
/// takes linear time overall. Going backwards restarts counting from `floor`.
#[derive(Clone, Copy)]
pub(crate) struct Cursor {
    ascii: bool,
    floor: (usize, usize),
    pos: (usize, usize),
}

impl Cursor {
    pub fn chars(&mut self, text: &str, byte: usize) -> usize {
        if self.ascii {
            return byte;
        }
        if byte < self.pos.0 {
            self.pos = self.floor;
        }
        self.pos.1 += text[self.pos.0..byte].chars().count();
        self.pos.0 = byte;
        self.pos.1
    }

    pub fn span(&mut self, text: &str, byte_start: usize, byte_end: usize) -> Span {
        let start = self.chars(text, byte_start);
        let end = self.chars(text, byte_end);
        Span {
            byte_start,
            byte_end,
            start,
            end,
        }
    }

    /// A cursor for the groups of a match starting at `span`, never counting from before it.
    pub fn for_groups(&self, span: Span) -> Cursor {
        Cursor {
            ascii: self.ascii,
            floor: (span.byte_start, span.start),
            pos: (span.byte_start, span.start),
        }
    }
}
