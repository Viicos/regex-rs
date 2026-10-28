use std::hash::{DefaultHasher, Hash, Hasher};

use pyo3::exceptions::{PyIndexError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyType};

use crate::errors::to_py_err;
use crate::haystack::{Haystack, search};
use crate::options::{Options, configure};

/// Match multiple, possibly overlapping, regexes in a single search.
///
/// A regex set corresponds to the union of zero or more regular expressions. That is, a regex set will
/// match a haystack when at least one of its constituent regexes matches. A regex set as its
/// formulated here provides a touch more power: it will also report *which* regular expressions in
/// the set match. Indeed, this is the key difference between regex sets and a single :class:`Regex`
/// with many alternates, since only one alternate can match at a time.
///
/// For example, consider regular expressions to match email addresses and domains:
/// ``[a-z]+@[a-z]+\.(com|org|net)`` and ``[a-z]+\.(com|org|net)``. If a regex set is constructed
/// from those regexes, then searching the haystack ``foo@example.com`` will report both regexes as
/// matching. Of course, one could accomplish this by compiling each regex on its own and doing two
/// searches over the haystack. The key advantage of using a regex set is that it will report the
/// matching regexes using a *single pass through the haystack*. If one has hundreds or thousands of
/// regexes to match repeatedly (like a URL router for a complex web application or a user agent
/// matcher), then a regex set *can* realize huge performance gains.
///
/// Create a new regex set with the given regular expressions. If any of the patterns are not valid
/// regular expressions, then an error is raised.
///
/// Args:
///     patterns: An iterable of the regular expressions to compile.
///     unicode: See :class:`Regex`.
///     case_insensitive: See :class:`Regex`.
///     multi_line: See :class:`Regex`.
///     dot_matches_new_line: See :class:`Regex`.
///     crlf: See :class:`Regex`.
///     line_terminator: See :class:`Regex`.
///     swap_greed: See :class:`Regex`.
///     ignore_whitespace: See :class:`Regex`.
///     octal: See :class:`Regex`.
///     size_limit: See :class:`Regex`.
///     dfa_size_limit: See :class:`Regex`.
///     nest_limit: See :class:`Regex`.
///
/// Raises:
///     PatternSyntaxError: If any of the patterns is invalid.
///     CompiledTooBigError: If the patterns are valid, but would produce a regex set that is bigger
///         than the configured size limit via ``size_limit``.
///
/// Limitations:
///     Regex sets are limited to answering the following two questions:
///
///     1. Does any regex in the set match?
///     2. If so, which regexes in the set match?
///
///     As with the main :class:`Regex` type, it is cheaper to ask (1) instead of (2) since the matching
///     engines can stop after the first match is found.
///
///     You cannot directly extract :class:`Match` or :class:`Captures` objects from a regex set. If
///     you need these operations, the recommended approach is to compile each pattern in the set
///     independently and scan the exact same haystack a second time with those independently compiled
///     patterns:
///
///     >>> # Compile a set matching any of our patterns.
///     >>> regex_set = RegexSet(["foo", "bar"])
///     >>> # Compile each pattern independently.
///     >>> regexes = [Regex(pattern) for pattern in regex_set.patterns]
///     >>> # Both patterns will match different ranges of this string.
///     >>> haystack = "barfoo"
///     >>> [regexes[index].find(haystack).text for index in regex_set.matches(haystack)]
///     ['foo', 'bar']
///
///     Matches arrive in the order the constituent patterns were declared, not the order they appear
///     in the haystack.
///
/// Performance:
///     A :class:`RegexSet` has the same performance characteristics as :class:`Regex`. Namely, search
///     takes ``O(m * n)`` time, where ``m`` is proportional to the size of the regex set and ``n`` is
///     proportional to the length of the haystack.
///
/// Examples:
///     This shows how the above two regexes (for matching email addresses and domains) might work:
///
///     >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
///     >>> # Ask whether any regexes in the set match.
///     >>> regex_set.is_match("foo@example.com")
///     True
///     >>> # Identify which regexes in the set match.
///     >>> list(regex_set.matches("foo@example.com"))
///     [0, 1]
///     >>> # Try again, but with a haystack that only matches one of the regexes.
///     >>> list(regex_set.matches("example.com"))
///     [1]
///     >>> # Try again, but with a haystack that doesn't match any regex in the set.
///     >>> list(regex_set.matches("example"))
///     []
///
///     Note that it would be possible to adapt the above example to using :class:`Regex` with an
///     expression like:
///
///     .. code-block:: regex
///
///         (?P<email>[a-z]+@(?P<email_domain>[a-z]+[.](com|org|net)))|(?P<domain>[a-z]+[.](com|org|net))
///
///     After a match, one could then inspect the capture groups to figure out which alternates
///     matched. The problem is that it is hard to make this approach scale when there are many regexes
///     since the overlap between each alternate isn't always obvious to reason about.
///
///     Create a new regex set from an iterable of strings:
///
///     >>> RegexSet([r"\w+", r"\d+"]).is_match("foo")
///     True
#[pyclass(frozen, module = "rustregex")]
pub struct RegexSet {
    inner: regex::RegexSet,
    options: Options,
}

#[pymethods]
impl RegexSet {
    #[new]
    #[pyo3(signature = (
        patterns: "collections.abc.Iterable[str]",
        /,
        *,
        unicode = true,
        case_insensitive = false,
        multi_line = false,
        dot_matches_new_line = false,
        crlf = false,
        line_terminator = "\n",
        swap_greed = false,
        ignore_whitespace = false,
        octal = false,
        size_limit = None,
        dfa_size_limit = None,
        nest_limit = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        py: Python<'_>,
        patterns: &Bound<'_, PyAny>,
        unicode: bool,
        case_insensitive: bool,
        multi_line: bool,
        dot_matches_new_line: bool,
        crlf: bool,
        line_terminator: &str,
        swap_greed: bool,
        ignore_whitespace: bool,
        octal: bool,
        size_limit: Option<usize>,
        dfa_size_limit: Option<usize>,
        nest_limit: Option<u32>,
    ) -> PyResult<Self> {
        // A string is an iterable of strings, but almost certainly not what was meant.
        if patterns.is_instance_of::<PyString>() {
            return Err(PyTypeError::new_err(
                "patterns must be an iterable of strings, not a string",
            ));
        }
        let patterns = patterns
            .try_iter()?
            .map(|p| p?.extract::<String>())
            .collect::<PyResult<Vec<_>>>()?;
        let options = Options::new(
            unicode,
            case_insensitive,
            multi_line,
            dot_matches_new_line,
            crlf,
            line_terminator,
            swap_greed,
            ignore_whitespace,
            octal,
            size_limit,
            dfa_size_limit,
            nest_limit,
        )?;
        let mut builder = regex::RegexSetBuilder::new(patterns);
        let inner = configure!(&mut builder, &options)
            .build()
            .map_err(|err| to_py_err(py, err))?;
        Ok(Self { inner, options })
    }

    /// Create a new empty regex set.
    ///
    /// An empty regex never matches anything.
    ///
    /// Example:
    ///     >>> regex_set = RegexSet.empty()
    ///     >>> len(regex_set)
    ///     0
    ///     >>> # An empty set matches nothing.
    ///     >>> regex_set.is_match("")
    ///     False
    #[classmethod]
    #[pyo3(signature = () -> "RegexSet")]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: regex::RegexSet::empty(),
            options: Options::default(),
        }
    }

    /// The regex patterns that this regex set was constructed from.
    ///
    /// This can be used to determine the pattern for a match. The list has exactly as many patterns given
    /// to this regex set, and the order of the list is the same as the order of the patterns provided to
    /// the set.
    ///
    /// Example:
    ///     >>> regex_set = RegexSet([r"\w+", r"\d+", r"\pL+", r"foo", r"bar", r"barfoo", r"foobar"])
    ///     >>> [regex_set.patterns[index] for index in regex_set.matches("foobar")]
    ///     ['\\w+', '\\pL+', 'foo', 'bar', 'foobar']
    #[getter]
    fn patterns(&self) -> Vec<&str> {
        self.inner.patterns().iter().map(String::as_str).collect()
    }

    /// Returns the total number of regexes in this set.
    ///
    /// Example:
    ///     >>> len(RegexSet.empty())
    ///     0
    ///     >>> len(RegexSet([r"[0-9]"]))
    ///     1
    ///     >>> len(RegexSet([r"[0-9]", r"[a-z]"]))
    ///     2
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Returns ``True`` if and only if one of the regexes in this set matches the haystack given.
    ///
    /// This method should be preferred if you only need to test whether any of the regexes in the set
    /// should match, but don't care about *which* regexes matched. This is because the underlying matching
    /// engine will quit immediately after seeing the first match instead of continuing to find all
    /// matches.
    ///
    /// Note that as with searches using :class:`Regex`, the expression is unanchored by default. That is,
    /// if the regex does not start with ``^`` or ``\A``, or end with ``$`` or ``\z``, then it is permitted
    /// to match anywhere in the haystack.
    ///
    /// Args:
    ///     haystack: The haystack to search.
    ///     start: The offset at which the search starts. The significance of the starting point is
    ///         that it takes the surrounding context into consideration. For example, the ``\A``
    ///         anchor can only match when ``start == 0``.
    ///
    /// Raises:
    ///     ValueError: When ``start`` is greater than the length of the haystack.
    ///
    /// Examples:
    ///     Tests whether a set matches somewhere in a haystack:
    ///
    ///     >>> regex_set = RegexSet([r"\w+", r"\d+"])
    ///     >>> regex_set.is_match("foo")
    ///     True
    ///     >>> regex_set.is_match("☃")
    ///     False
    ///
    ///     This example shows the significance of ``start``. Namely, consider a haystack ``foobar`` and a
    ///     desire to execute a search starting at offset ``3``. You could search a substring explicitly,
    ///     but then the look-around assertions won't work correctly. Instead, you can use ``start`` to
    ///     specify the start position of a search.
    ///
    ///     >>> regex_set = RegexSet([r"\bbar\b", r"(?m)^bar$"])
    ///     >>> # We get a match here, but it's probably not intended.
    ///     >>> regex_set.is_match("foobar"[3:])
    ///     True
    ///     >>> # No match because the assertions take the context into account.
    ///     >>> regex_set.is_match("foobar", 3)
    ///     False
    #[pyo3(signature = (haystack, /, start = 0))]
    fn is_match(
        &self,
        py: Python<'_>,
        haystack: &Bound<'_, PyString>,
        start: usize,
    ) -> PyResult<bool> {
        let haystack = Haystack::new(haystack)?;
        let start = haystack.byte_offset(start)?;
        let (set, text) = (&self.inner, haystack.text);
        Ok(search(py, text.len(), || set.is_match_at(text, start)))
    }

    /// Returns the set of regexes that match in the given haystack.
    ///
    /// The set returned contains the index of each regex that matches in the given haystack. The index is
    /// in correspondence with the order of regular expressions given to the :class:`RegexSet`
    /// constructor.
    ///
    /// The set can also be used to iterate over the matched indices. The order of iteration is always
    /// ascending with respect to the matching indices.
    ///
    /// Note that as with searches using :class:`Regex`, the expression is unanchored by default. That is,
    /// if the regex does not start with ``^`` or ``\A``, or end with ``$`` or ``\z``, then it is permitted
    /// to match anywhere in the haystack.
    ///
    /// Args:
    ///     haystack: The haystack to search.
    ///     start: The offset at which the search starts. The significance of the starting point is
    ///         that it takes the surrounding context into consideration. For example, the ``\A``
    ///         anchor can only match when ``start == 0``.
    ///
    /// Raises:
    ///     ValueError: When ``start`` is greater than the length of the haystack.
    ///
    /// Examples:
    ///     Tests which regular expressions match the given haystack:
    ///
    ///     >>> regex_set = RegexSet([r"\w+", r"\d+", r"\pL+", r"foo", r"bar", r"barfoo", r"foobar"])
    ///     >>> list(regex_set.matches("foobar"))
    ///     [0, 2, 3, 4, 6]
    ///
    ///     You can also test whether a particular regex matched:
    ///
    ///     >>> matches = regex_set.matches("foobar")
    ///     >>> matches.matched(5)
    ///     False
    ///     >>> matches.matched(6)
    ///     True
    ///
    ///     This example shows the significance of ``start``:
    ///
    ///     >>> regex_set = RegexSet([r"\bbar\b", r"(?m)^bar$"])
    ///     >>> # We get matches here, but it's probably not intended.
    ///     >>> list(regex_set.matches("foobar"[3:]))
    ///     [0, 1]
    ///     >>> # No matches because the assertions take the context into account.
    ///     >>> list(regex_set.matches("foobar", 3))
    ///     []
    #[pyo3(signature = (haystack, /, start = 0))]
    fn matches(
        &self,
        py: Python<'_>,
        haystack: &Bound<'_, PyString>,
        start: usize,
    ) -> PyResult<SetMatches> {
        let haystack = Haystack::new(haystack)?;
        let start = haystack.byte_offset(start)?;
        let (set, text) = (&self.inner, haystack.text);
        Ok(SetMatches {
            inner: search(py, text.len(), || set.matches_at(text, start)),
        })
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let patterns = PyList::new(py, self.inner.patterns())?.repr()?;
        Ok(format!(
            "RegexSet({patterns}{})",
            self.options.repr_kwargs(py)?
        ))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.patterns() == other.inner.patterns() && self.options == other.options
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        (self.inner.patterns(), &self.options).hash(&mut hasher);
        hasher.finish()
    }

    #[pyo3(signature = () -> "tuple[tuple[list[str]], dict[str, object]]")]
    fn __getnewargs_ex__<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<((Vec<String>,), Bound<'py, PyDict>)> {
        Ok((
            (self.inner.patterns().to_vec(),),
            self.options.to_kwargs(py, false)?,
        ))
    }

    #[pyo3(signature = () -> "RegexSet")]
    fn __copy__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (memo, /) -> "RegexSet")]
    fn __deepcopy__<'py>(slf: PyRef<'py, Self>, memo: &Bound<'py, PyAny>) -> PyRef<'py, Self> {
        let _ = memo;
        slf
    }
}

/// A set of matches returned by a regex set.
///
/// Iterating over it yields the indices of the regexes that matched, in ascending order, and its
/// length is the number of regexes that matched.
///
/// Values of this type are constructed by :meth:`RegexSet.matches`.
#[pyclass(frozen, module = "rustregex")]
pub struct SetMatches {
    inner: regex::SetMatches,
}

#[pymethods]
impl SetMatches {
    /// Whether this set contains any matches.
    ///
    /// Example:
    ///     >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
    ///     >>> regex_set.matches("foo@example.com").matched_any()
    ///     True
    fn matched_any(&self) -> bool {
        self.inner.matched_any()
    }

    /// Whether all patterns in this set matched.
    ///
    /// Example:
    ///     >>> regex_set = RegexSet([r"^foo", r"[a-z]+\.com"])
    ///     >>> regex_set.matches("foo.example.com").matched_all()
    ///     True
    fn matched_all(&self) -> bool {
        self.inner.matched_all()
    }

    /// Whether the regex at the given index matched.
    ///
    /// The index for a regex is determined by its insertion order upon the initial construction of a
    /// :class:`RegexSet`, starting at ``0``.
    ///
    /// Raises:
    ///     IndexError: If ``index`` is greater than or equal to the number of regexes in the original set
    ///         that produced these matches.
    ///
    /// Example:
    ///     >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
    ///     >>> matches = regex_set.matches("example.com")
    ///     >>> matches.matched(0)
    ///     False
    ///     >>> matches.matched(1)
    ///     True
    fn matched(&self, index: usize) -> PyResult<bool> {
        if index >= self.inner.len() {
            return Err(PyIndexError::new_err("regex index out of range"));
        }
        Ok(self.inner.matched(index))
    }

    fn __contains__(&self, index: usize) -> bool {
        index < self.inner.len() && self.inner.matched(index)
    }

    fn __len__(&self) -> usize {
        self.inner.iter().count()
    }

    fn __iter__(&self) -> SetMatchesIter {
        SetMatchesIter {
            indices: self.inner.clone().into_iter(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "<rustregex.SetMatches {:?}>",
            self.inner.iter().collect::<Vec<_>>()
        )
    }
}

/// An iterator over the set of matches from a regex set.
///
/// This will always produces matches in ascending order of index, where the index corresponds to the
/// index of the regex that matched with respect to its position when initially building the set.
///
/// This iterator is created by iterating over a :class:`SetMatches`, which is automatically done in
/// ``for`` loops.
///
/// Example:
///     >>> regex_set = RegexSet([r"[0-9]", r"[a-z]", r"[A-Z]", r"\p{Greek}"])
///     >>> [index for index in regex_set.matches("βa1")]
///     [0, 1, 3]
#[pyclass(module = "rustregex")]
pub struct SetMatchesIter {
    indices: regex::SetMatchesIntoIter,
}

#[pymethods]
impl SetMatchesIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<usize> {
        self.indices.next()
    }
}
