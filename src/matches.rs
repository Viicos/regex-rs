use pyo3::exceptions::{PyIndexError, PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};

use crate::haystack::{Cursor, Haystack, Span};
use crate::pyregex::Regex;

/// Returns the text of `span` in `haystack`, reusing the haystack object if the span covers all of it.
fn slice<'py>(haystack: &Bound<'py, PyString>, span: Span) -> PyResult<Bound<'py, PyString>> {
    let text = haystack.to_str()?;
    if span.byte_start == 0 && span.byte_end == text.len() {
        return Ok(haystack.clone());
    }
    Ok(PyString::new(
        haystack.py(),
        &text[span.byte_start..span.byte_end],
    ))
}

/// A capture group index or name.
#[derive(FromPyObject)]
enum GroupKey {
    Index(isize),
    Name(String),
}

/// Represents a single match of a regex in a haystack.
///
/// A :class:`Match` contains both the start and end offsets of the match and the actual substring
/// corresponding to the range of those offsets. It is guaranteed that ``start <= end``. When
/// ``start == end``, the match is empty.
///
/// Values with this type are created by :meth:`Regex.find` or :meth:`Regex.find_iter`. Other APIs can
/// create :class:`Match` values too. For example, :meth:`Captures.get`.
///
/// Numbering:
///     The offsets in a :class:`Match` form a half-open interval. That is, the start of the range is
///     inclusive and the end of the range is exclusive. For example, given a haystack ``abcFOOxyz``
///     and a match of ``FOO``, its offset range starts at ``3`` and ends at ``6``. ``3`` corresponds
///     to ``F`` and ``6`` corresponds to ``x``, which is one past the end of the match. This
///     corresponds to the same kind of slicing that Python uses.
///
/// Example:
///     This example shows the value of each of the attributes and methods on :class:`Match` for a
///     particular search.
///
///     >>> m = Regex(r"\p{Greek}+").find("Greek: αβγδ")
///     >>> m.start, m.end, m.is_empty(), len(m), m.span, m.text
///     (7, 11, False, 4, (7, 11), 'αβγδ')
#[pyclass(frozen, module = "rustregex")]
pub struct Match {
    haystack: Py<PyString>,
    span: Span,
}

impl Match {
    pub(crate) fn new(py: Python<'_>, haystack: &Py<PyString>, span: Span) -> Self {
        Self {
            haystack: haystack.clone_ref(py),
            span,
        }
    }
}

#[pymethods]
impl Match {
    /// The offset of the start of the match in the haystack. The start of the match corresponds to the
    /// position where the match begins and includes the first character in the match.
    ///
    /// It is guaranteed that ``start <= end``.
    #[getter]
    fn start(&self) -> usize {
        self.span.start
    }

    /// The offset of the end of the match in the haystack. The end of the match corresponds to the
    /// character immediately following the last character in the match. This means that
    /// ``haystack[start:end]`` works as one would expect.
    ///
    /// It is guaranteed that ``start <= end``.
    #[getter]
    fn end(&self) -> usize {
        self.span.end
    }

    /// The starting and ending offsets of the match in the haystack.
    ///
    /// It is always correct to slice the original haystack searched with these offsets.
    #[getter]
    fn span(&self) -> (usize, usize) {
        (self.span.start, self.span.end)
    }

    /// The substring of the haystack that matched.
    #[getter]
    fn text<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        slice(self.haystack.bind(py), self.span)
    }

    /// Returns ``True`` if and only if this match has a length of zero.
    ///
    /// Note that an empty match can only occur when the regex itself can match the empty string. Here are
    /// some examples of regexes that can all match the empty string: ``^``, ``^$``, ``\b``, ``a?``,
    /// ``a*``, ``a{0}``, ``(foo|\d+|quux)?``.
    fn is_empty(&self) -> bool {
        self.span.start == self.span.end
    }

    /// The length, in characters, of this match.
    fn __len__(&self) -> usize {
        self.span.end - self.span.start
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let text = self.text(py)?.repr()?;
        Ok(format!(
            "<rustregex.Match span=({}, {}) text={text}>",
            self.span.start, self.span.end
        ))
    }
}

/// Represents the capture groups for a single match.
///
/// Capture groups refer to parts of a regex enclosed in parentheses. They can be optionally named. The
/// purpose of capture groups is to be able to reference different parts of a match based on the
/// original pattern. In essence, a :class:`Captures` is a container of :class:`Match` values for each
/// group that participated in a regex match. Each :class:`Match` can be looked up by either its
/// capture group index or name (if it has one).
///
/// For example, say you want to match the individual letters in a 5-letter word:
///
/// .. code-block:: regex
///
///     (?<first>\w)(\w)(?:\w)\w(?<last>\w)
///
/// This regex has 4 capture groups:
///
/// * The group at index ``0`` corresponds to the overall match. It is always present in every match
///   and never has a name.
/// * The group at index ``1`` with name ``first`` corresponding to the first letter.
/// * The group at index ``2`` with no name corresponding to the second letter.
/// * The group at index ``3`` with name ``last`` corresponding to the fifth and last letter.
///
/// Notice that ``(?:\w)`` was not listed above as a capture group despite it being enclosed in
/// parentheses. That's because ``(?:pattern)`` is a special syntax that permits grouping but *without*
/// capturing. The reason for not treating it as a capture is that tracking and reporting capture
/// groups requires additional state that may lead to slower searches. So using as few capture groups
/// as possible can help performance. (Although the difference in performance of a couple of capture
/// groups is likely immaterial.)
///
/// Values with this type are created by :meth:`Regex.captures` or :meth:`Regex.captures_iter`.
///
/// Example:
///     >>> caps = Regex(r"(?<first>\w)(\w)(?:\w)\w(?<last>\w)").captures("toady")
///     >>> caps[0].text, caps["first"].text, caps[2].text, caps["last"].text
///     ('toady', 't', 'o', 'y')
#[pyclass(frozen, module = "rustregex")]
pub struct Captures {
    regex: Py<Regex>,
    haystack: Py<PyString>,
    groups: Vec<Option<Span>>,
}

impl Captures {
    /// Builds a `Captures` from the capture groups found by the `regex` crate.
    ///
    /// `cursor` converts the offsets of the overall match, and must not be past its start.
    pub(crate) fn new(
        py: Python<'_>,
        regex: &Py<Regex>,
        haystack: &Haystack<'_, '_>,
        caps: &regex::Captures<'_>,
        cursor: &mut Cursor,
    ) -> Self {
        let whole = caps.get_match();
        let whole = cursor.span(haystack.text, whole.start(), whole.end());
        let mut group_cursor = cursor.for_groups(whole);
        let groups = caps
            .iter()
            .enumerate()
            .map(|(index, m)| match (index, m) {
                (0, _) => Some(whole),
                (_, Some(m)) => Some(group_cursor.span(haystack.text, m.start(), m.end())),
                (_, None) => None,
            })
            .collect();
        Self {
            regex: regex.clone_ref(py),
            haystack: haystack.obj.clone().unbind(),
            groups,
        }
    }

    fn group(&self, py: Python<'_>, index: usize) -> Option<Match> {
        let span = (*self.groups.get(index)?)?;
        Some(Match::new(py, &self.haystack, span))
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.regex
            .get()
            .inner
            .capture_names()
            .position(|n| n == Some(name))
    }

    fn participating(&self) -> impl Iterator<Item = Span> + '_ {
        self.groups.iter().filter_map(|span| *span)
    }
}

#[pymethods]
impl Captures {
    /// Returns the :class:`Match` associated with the capture group at index ``index``. If ``index`` does
    /// not correspond to a capture group, or if the capture group did not participate in the match, then
    /// ``None`` is returned.
    ///
    /// When ``index == 0``, this is guaranteed to return a non-``None`` value.
    ///
    /// Example:
    ///     Get the substring that matched with a default of an empty string if the group didn't
    ///     participate in the match:
    ///
    ///     >>> caps = Regex(r"[a-z]+(?:([0-9]+)|([A-Z]+))").captures("abc123")
    ///     >>> m1, m2 = caps.get(1), caps.get(2)
    ///     >>> m1.text if m1 else "", m2.text if m2 else ""
    ///     ('123', '')
    fn get(&self, py: Python<'_>, index: usize) -> Option<Match> {
        self.group(py, index)
    }

    /// Returns the :class:`Match` associated with the capture group named ``name``. If ``name`` isn't a
    /// valid capture group or it refers to a group that didn't match, then ``None`` is returned.
    ///
    /// Example:
    ///     Get the substring that matched with a default of an empty string if the group didn't
    ///     participate in the match:
    ///
    ///     >>> caps = Regex(r"[a-z]+(?:(?<numbers>[0-9]+)|(?<letters>[A-Z]+))").captures("abc123")
    ///     >>> numbers, letters = caps.name("numbers"), caps.name("letters")
    ///     >>> numbers.text if numbers else "", letters.text if letters else ""
    ///     ('123', '')
    fn name(&self, py: Python<'_>, name: &str) -> Option<Match> {
        self.group(py, self.index_of(name)?)
    }

    /// Return the overall match for the capture.
    ///
    /// This returns the match for index ``0``. That is it is equivalent to ``caps.get(0)``, except that
    /// the return value is never ``None``.
    ///
    /// Example:
    ///     >>> Regex(r"[a-z]+([0-9]+)").captures("   abc123-def").get_match().text
    ///     'abc123'
    fn get_match(&self, py: Python<'_>) -> Match {
        Match::new(
            py,
            &self.haystack,
            self.groups[0].expect("group 0 always participates"),
        )
    }

    /// Get a matching capture group by index or name.
    ///
    /// Unlike :meth:`get` and :meth:`name`, an error is raised for a group that doesn't exist. ``None`` is
    /// still returned for a group that didn't participate in the match.
    ///
    /// Raises:
    ///     IndexError: If there is no group at the given index.
    ///     KeyError: If there is no group with the given name.
    fn __getitem__(&self, py: Python<'_>, key: GroupKey) -> PyResult<Option<Match>> {
        let index = match key {
            GroupKey::Index(index) => match usize::try_from(index) {
                Ok(index) if index < self.groups.len() => index,
                _ => return Err(PyIndexError::new_err("no such group")),
            },
            GroupKey::Name(name) => self
                .index_of(&name)
                .ok_or_else(|| PyKeyError::new_err(name))?,
        };
        Ok(self.group(py, index))
    }

    /// Returns the total number of capture groups. This includes both matching and non-matching groups.
    ///
    /// The length returned is always equivalent to the number of elements yielded when iterating over
    /// this :class:`Captures`. Consequently, the length is always greater than zero since every
    /// :class:`Captures` value always includes the match for the entire regex.
    ///
    /// Example:
    ///     >>> len(Regex(r"(\w)(\d)?(\w)").captures("AZ"))
    ///     4
    fn __len__(&self) -> usize {
        self.groups.len()
    }

    /// Returns an iterator over all capture groups. This includes both matching and non-matching groups.
    ///
    /// The iterator always yields at least one matching group: the first group (at index ``0``) with no
    /// name. Subsequent groups are returned in the order of their opening parenthesis in the regex.
    ///
    /// The elements yielded have type ``Match | None``, where a non-``None`` value is present if the
    /// capture group matches.
    ///
    /// Example:
    ///     >>> caps = Regex(r"(\w)(\d)?(\w)").captures("AZ")
    ///     >>> [m.text if m else None for m in caps]
    ///     ['AZ', 'A', None, 'Z']
    fn __iter__(&self, py: Python<'_>) -> SubCaptureMatches {
        let groups = (0..self.groups.len())
            .map(|index| self.group(py, index))
            .collect::<Vec<_>>();
        SubCaptureMatches {
            groups: groups.into_iter(),
        }
    }

    /// This is a convenience routine for extracting the substrings corresponding to matching capture
    /// groups.
    ///
    /// This returns a tuple where the first element corresponds to the full substring of the haystack
    /// that matched the regex. The second element is a tuple of substrings, with each corresponding to the
    /// substring that matched for a particular capture group.
    ///
    /// This only works when the number of possible matching groups is fixed in all circumstances. More
    /// precisely, this routine only works when :attr:`Regex.static_captures_len` is not ``None``.
    ///
    /// Stated more plainly, if the number of matching capture groups in a regex can vary from match to
    /// match, then this function always raises an error.
    ///
    /// For example, ``(a)(b)|(c)`` could produce two matching capture groups or one matching capture group
    /// for any given match. Therefore, one cannot use ``extract`` with such a pattern.
    ///
    /// But a pattern like ``(a)(b)|(c)(d)`` can be used with ``extract`` because the number of capture
    /// groups in every match is always equivalent, even if the capture *indices* in each match are not.
    ///
    /// Raises:
    ///     ValueError: If the number of matching capture groups can vary from match to match.
    ///
    /// Examples:
    ///     >>> regex = Regex(r"([0-9]{4})-([0-9]{2})-([0-9]{2})")
    ///     >>> full, (year, month, day) = regex.captures("On 2010-03-14, I became a Tennessee lamb.").extract()
    ///     >>> full, year, month, day
    ///     ('2010-03-14', '2010', '03', '14')
    ///
    ///     This example shows how to use this method when iterating over all :class:`Captures` matches in
    ///     a haystack.
    ///
    ///     >>> haystack = "1973-01-05, 1975-08-25 and 1980-10-18"
    ///     >>> [groups for _, groups in (caps.extract() for caps in regex.captures_iter(haystack))]
    ///     [('1973', '01', '05'), ('1975', '08', '25'), ('1980', '10', '18')]
    ///
    ///     This API is particularly useful when you need to extract a particular value that might occur in
    ///     a different format. Consider, for example, an identifier that might be in double quotes or
    ///     single quotes:
    ///
    ///     >>> regex = Regex('id:(?:"([^"]+)"|' + "'([^']+)')")
    ///     >>> haystack = 'The first is id:"foo" and the second is ' + "id:'bar'."
    ///     >>> [id_ for _, (id_,) in (caps.extract() for caps in regex.captures_iter(haystack))]
    ///     ['foo', 'bar']
    #[pyo3(signature = () -> "tuple[str, tuple[str, ...]]")]
    fn extract<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Bound<'py, PyString>, Bound<'py, PyTuple>)> {
        if self.regex.get().inner.static_captures_len().is_none() {
            return Err(PyValueError::new_err(
                "extract() can't be used, as the number of participating groups can vary from match to match",
            ));
        }
        let haystack = self.haystack.bind(py);
        let mut texts = self.participating().map(|span| slice(haystack, span));
        let whole = texts.next().expect("group 0 always participates")?;
        let groups = texts.collect::<PyResult<Vec<_>>>()?;
        Ok((whole, PyTuple::new(py, groups)?))
    }

    /// Expands all instances of ``$ref`` in ``replacement`` to the corresponding capture group, and
    /// returns the result. A ``ref`` can be a capture group index or a name. If ``ref`` doesn't refer to a
    /// capture group that participated in the match, then it is replaced with the empty string.
    ///
    /// Format:
    ///     The format of the replacement string supports two different kinds of capture references:
    ///     unbraced and braced.
    ///
    ///     For the unbraced format, the format supported is ``$ref`` where ``name`` can be any character
    ///     in the class ``[0-9A-Za-z_]``. ``ref`` is always the longest possible parse. So for example,
    ///     ``$1a`` corresponds to the capture group named ``1a`` and not the capture group at index ``1``.
    ///     If ``ref`` matches ``^[0-9]+$``, then it is treated as a capture group index itself and not a
    ///     name.
    ///
    ///     For the braced format, the format supported is ``${ref}`` where ``ref`` can be any sequence of
    ///     characters except for ``}``. If no closing brace occurs, then it is not considered a capture
    ///     reference. As with the unbraced format, if ``ref`` matches ``^[0-9]+$``, then it is treated as
    ///     a capture group index and not a name.
    ///
    ///     The braced format is useful for exerting precise control over the name of the capture
    ///     reference. For example, ``${1}a`` corresponds to the capture group reference ``1`` followed by
    ///     the letter ``a``, where as ``$1a`` (as mentioned above) corresponds to the capture group
    ///     reference ``1a``. The braced format is also useful for expressing capture group names that use
    ///     characters not supported by the unbraced format. For example, ``${foo[bar].baz}`` refers to the
    ///     capture group named ``foo[bar].baz``.
    ///
    ///     If a capture group reference is found and it does not refer to a valid capture group, then it
    ///     will be replaced with the empty string.
    ///
    ///     To write a literal ``$``, use ``$$``.
    ///
    /// Example:
    ///     >>> regex = Regex(r"(?<day>[0-9]{2})-(?<month>[0-9]{2})-(?<year>[0-9]{4})")
    ///     >>> caps = regex.captures("On 14-03-2010, I became a Tennessee lamb.")
    ///     >>> caps.expand("year=$year, month=$month, day=$day")
    ///     'year=2010, month=03, day=14'
    fn expand(&self, py: Python<'_>, replacement: &str) -> PyResult<String> {
        let haystack = self.haystack.bind(py).to_str()?;
        let mut out = String::new();
        regex_automata::util::interpolate::string(
            replacement,
            |index, out| {
                if let Some(Some(span)) = self.groups.get(index) {
                    out.push_str(&haystack[span.byte_start..span.byte_end]);
                }
            },
            |name| self.index_of(name),
            &mut out,
        );
        Ok(out)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let whole = self.get_match(py);
        let text = whole.text(py)?.repr()?;
        Ok(format!(
            "<rustregex.Captures span=({}, {}) text={text}>",
            whole.span.start, whole.span.end
        ))
    }
}

/// An iterator over all group matches in a :class:`Captures` value.
///
/// This iterator yields values of type ``Match | None``. The order of elements yielded corresponds to
/// the order of the opening parenthesis for the group in the regex pattern. ``None`` is yielded for
/// groups that did not participate in the match.
///
/// The first element always corresponds to the implicit group for the overall match. Since this
/// iterator is created by a :class:`Captures` value, and a :class:`Captures` value is only created
/// when a match occurs, it follows that the first element yielded by this iterator is guaranteed to
/// be non-``None``.
///
/// This iterator is created by iterating over a :class:`Captures`.
#[pyclass(module = "rustregex")]
pub struct SubCaptureMatches {
    groups: std::vec::IntoIter<Option<Match>>,
}

#[pymethods]
impl SubCaptureMatches {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Option<Match>> {
        self.groups.next()
    }
}
