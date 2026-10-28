use pyo3::prelude::*;
use pyo3::types::PyString;

use crate::haystack::{Cursor, Haystack, search};
use crate::matches::{Captures, Match};
use crate::pyregex::Regex;

/// The state of an iteration over successive non-overlapping matches.
///
/// The iterators of the `regex` crate borrow the haystack, which can't be stored in a Python object.
/// This reproduces their behavior by running one search per step, including the handling of an
/// empty match found right where the previous match ended: such a match is skipped by searching
/// again one byte further.
pub(crate) struct Searcher {
    start: usize,
    last_match_end: Option<usize>,
    done: bool,
}

impl Searcher {
    pub fn new() -> Self {
        Self {
            start: 0,
            last_match_end: None,
            done: false,
        }
    }

    /// Runs `find` (a search starting at the given byte offset) to get the next match, if any.
    pub fn next<T>(
        &mut self,
        haystack_len: usize,
        mut find: impl FnMut(usize) -> Option<T>,
        span: impl Fn(&T) -> (usize, usize),
    ) -> Option<T> {
        if self.done {
            return None;
        }
        let mut found = find(self.start);
        if let Some(m) = &found {
            let (start, end) = span(m);
            if start == end && Some(end) == self.last_match_end {
                self.start += 1;
                found = if self.start > haystack_len {
                    None
                } else {
                    find(self.start)
                };
            }
        }
        match found {
            Some(m) => {
                let (_, end) = span(&m);
                self.start = end;
                self.last_match_end = Some(end);
                Some(m)
            }
            None => {
                self.done = true;
                None
            }
        }
    }
}

/// An iterator over all non-overlapping matches in a haystack.
///
/// This iterator yields :class:`Match` values. The iterator stops when no more matches can be found.
///
/// This iterator is created by :meth:`Regex.find_iter`.
///
/// Time complexity:
///     Note that since an iterator runs potentially many searches on the haystack and since each search
///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
///     iteration is ``O(m * n^2)``.
#[pyclass(module = "regex_rs")]
pub struct Matches {
    regex: Py<Regex>,
    haystack: Py<PyString>,
    searcher: Searcher,
    cursor: Cursor,
}

impl Matches {
    pub(crate) fn new(regex: Py<Regex>, haystack: &Haystack<'_, '_>) -> Self {
        Self {
            regex,
            haystack: haystack.obj.clone().unbind(),
            searcher: Searcher::new(),
            cursor: haystack.cursor_at(0, 0),
        }
    }
}

#[pymethods]
impl Matches {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Match>> {
        let haystack = self.haystack.bind(py);
        let text = haystack.to_str()?;
        let regex = &self.regex.get().inner;
        let found = self.searcher.next(
            text.len(),
            |start| search(py, text.len(), || regex.find_at(text, start)),
            |m| (m.start(), m.end()),
        );
        Ok(found.map(|m| {
            Match::new(
                py,
                &self.haystack,
                self.cursor.span(text, m.start(), m.end()),
            )
        }))
    }
}

/// An iterator over all non-overlapping capture matches in a haystack.
///
/// This iterator yields :class:`Captures` values. The iterator stops when no more matches can be
/// found.
///
/// This iterator is created by :meth:`Regex.captures_iter`.
///
/// Time complexity:
///     Note that since an iterator runs potentially many searches on the haystack and since each search
///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
///     iteration is ``O(m * n^2)``.
#[pyclass(module = "regex_rs")]
pub struct CaptureMatches {
    regex: Py<Regex>,
    haystack: Py<PyString>,
    searcher: Searcher,
    cursor: Cursor,
}

impl CaptureMatches {
    pub(crate) fn new(regex: Py<Regex>, haystack: &Haystack<'_, '_>) -> Self {
        Self {
            regex,
            haystack: haystack.obj.clone().unbind(),
            searcher: Searcher::new(),
            cursor: haystack.cursor_at(0, 0),
        }
    }
}

#[pymethods]
impl CaptureMatches {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Captures>> {
        let haystack = self.haystack.bind(py);
        let haystack = Haystack::new(haystack)?;
        let text = haystack.text;
        let regex = &self.regex.get().inner;
        let found = self.searcher.next(
            text.len(),
            |start| search(py, text.len(), || regex.captures_at(text, start)),
            |caps| {
                let m = caps.get_match();
                (m.start(), m.end())
            },
        );
        Ok(found.map(|caps| Captures::new(py, &self.regex, &haystack, &caps, &mut self.cursor)))
    }
}

/// An iterator over all substrings delimited by a regex match.
///
/// This iterator is created by :meth:`Regex.split`.
///
/// Time complexity:
///     Note that since an iterator runs potentially many searches on the haystack and since each search
///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
///     iteration is ``O(m * n^2)``.
#[pyclass(module = "regex_rs")]
pub struct Split {
    regex: Py<Regex>,
    haystack: Py<PyString>,
    searcher: Searcher,
    /// Byte offset where the next substring starts, past the end of the haystack once exhausted.
    last: usize,
}

impl Split {
    pub(crate) fn new(regex: Py<Regex>, haystack: &Haystack<'_, '_>) -> Self {
        Self {
            regex,
            haystack: haystack.obj.clone().unbind(),
            searcher: Searcher::new(),
            last: 0,
        }
    }

    fn next_piece<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        let haystack = self.haystack.bind(py);
        let text = haystack.to_str()?;
        let regex = &self.regex.get().inner;
        let found = self.searcher.next(
            text.len(),
            |start| search(py, text.len(), || regex.find_at(text, start)),
            |m| (m.start(), m.end()),
        );
        let piece = match found {
            Some(m) => {
                let piece = &text[self.last..m.start()];
                self.last = m.end();
                piece
            }
            None if self.last > text.len() => return Ok(None),
            None => {
                let piece = &text[self.last..];
                self.last = text.len() + 1;
                piece
            }
        };
        Ok(Some(PyString::new(py, piece)))
    }

    /// Returns everything that hasn't been yielded yet, if anything.
    fn remainder<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        let haystack = self.haystack.bind(py);
        let text = haystack.to_str()?;
        if self.last > text.len() {
            return Ok(None);
        }
        let piece = PyString::new(py, &text[self.last..]);
        self.last = text.len() + 1;
        Ok(Some(piece))
    }
}

#[pymethods]
impl Split {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        self.next_piece(py)
    }
}

/// An iterator over at most ``limit`` substrings delimited by a regex match.
///
/// The last substring yielded by this iterator will be whatever remains after ``limit - 1`` splits.
///
/// This iterator is created by :meth:`Regex.splitn`.
///
/// Time complexity:
///     Note that since an iterator runs potentially many searches on the haystack and since each search
///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
///     iteration is ``O(m * n^2)``.
///
///     Although note that the worst case time here has an upper bound given by the ``limit``
///     parameter to :meth:`Regex.splitn`.
#[pyclass(module = "regex_rs")]
pub struct SplitN {
    split: Split,
    limit: usize,
}

impl SplitN {
    pub(crate) fn new(regex: Py<Regex>, haystack: &Haystack<'_, '_>, limit: usize) -> Self {
        Self {
            split: Split::new(regex, haystack),
            limit,
        }
    }
}

#[pymethods]
impl SplitN {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyString>>> {
        if self.limit == 0 {
            return Ok(None);
        }
        self.limit -= 1;
        if self.limit > 0 {
            self.split.next_piece(py)
        } else {
            self.split.remainder(py)
        }
    }
}
