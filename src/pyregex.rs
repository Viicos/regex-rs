use std::borrow::Cow;
use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

use crate::errors::to_py_err;
use crate::haystack::{Haystack, search};
use crate::iter::{CaptureMatches, Matches, Split, SplitN};
use crate::matches::{Captures, Match};
use crate::options::{Options, configure};

/// A compiled regular expression for searching Unicode haystacks.
///
/// A :class:`Regex` can be used to search haystacks, split haystacks into substrings or replace
/// substrings in a haystack with a different substring. All searching is done with an implicit
/// ``(?s:.)*?`` at the beginning and end of a pattern. To force an expression to match the whole
/// string (or a prefix or a suffix), you must use an anchor like ``^`` or ``$`` (or ``\A`` and
/// ``\z``).
///
/// While this library will handle Unicode strings (whether in the regular expression or in the
/// haystack), all positions returned are offsets of characters in the haystack, which can be used to
/// slice it.
///
/// Once compiled, a regular expression can be used repeatedly to search, split or replace substrings in
/// a haystack.
///
/// Note that regex compilation tends to be a somewhat expensive process, and unlike higher level
/// environments, compilation is not automatically cached for you. One should endeavor to compile a
/// regex once and then reuse it. For example, it's a bad idea to compile the same regex repeatedly in
/// a loop.
///
/// Args:
///     pattern: The regular expression to compile.
///     unicode: This configures Unicode mode for the entire pattern.
///
///         Enabling Unicode mode does a number of things:
///
///         * Most fundamentally, it causes the fundamental atom of matching to be a single codepoint.
///           When Unicode mode is disabled, it's a single byte. For example, when Unicode mode is
///           enabled, ``.`` will match ``💩`` once, where as it will match 4 times when Unicode mode
///           is disabled. (Since the UTF-8 encoding of ``💩`` is 4 bytes long.)
///         * Case insensitive matching uses Unicode simple case folding rules.
///         * Unicode character classes like ``\p{Letter}`` and ``\p{Greek}`` are available.
///         * Perl character classes are Unicode aware. That is, ``\w``, ``\s`` and ``\d``.
///         * The word boundary assertions, ``\b`` and ``\B``, use the Unicode definition of a word
///           character.
///
///         Note that if Unicode mode is disabled, then the regex will fail to compile if it could
///         match invalid UTF-8. For example, when Unicode mode is disabled, then since ``.`` matches
///         any byte (except for ``\n``), then it can match invalid UTF-8 and thus building a regex
///         from it will fail. Another example is ``\w`` and ``\W``. Since ``\w`` can only match ASCII
///         bytes when Unicode mode is disabled, it's allowed. But ``\W`` can match more than ASCII
///         bytes, including invalid UTF-8, and so it is not allowed.
///
///         For more details on the Unicode support, see the `Unicode section
///         <https://docs.rs/regex/1.13.1/regex/#unicode>`__ of the documentation of the Rust
///         ``regex`` crate.
///
///         The default for this is ``True``.
///     case_insensitive: This configures whether to enable case insensitive matching for the entire
///         pattern.
///
///         This setting can also be configured using the inline flag ``i`` in the pattern. For
///         example, ``(?i:foo)`` matches ``foo`` case insensitively while ``(?-i:foo)`` matches
///         ``foo`` case sensitively.
///
///         The default for this is ``False``.
///     multi_line: This configures multi-line mode for the entire pattern.
///
///         Enabling multi-line mode changes the behavior of the ``^`` and ``$`` anchor assertions.
///         Instead of only matching at the beginning and end of a haystack, respectively, multi-line
///         mode causes them to match at the beginning and end of a line *in addition* to the
///         beginning and end of a haystack. More precisely, ``^`` will match at the position
///         immediately following a ``\n`` and ``$`` will match at the position immediately preceding
///         a ``\n``.
///
///         The behavior of this option can be impacted by other settings too:
///
///         * The ``line_terminator`` option changes ``\n`` above to any ASCII character.
///         * The ``crlf`` option changes the line terminator to be either ``\r`` or ``\n``, but never
///           at the position between a ``\r`` and ``\n``.
///
///         This setting can also be configured using the inline flag ``m`` in the pattern.
///
///         The default for this is ``False``.
///     dot_matches_new_line: This configures dot-matches-new-line mode for the entire pattern.
///
///         Perhaps surprisingly, the default behavior for ``.`` is not to match any character, but
///         rather, to match any character except for the line terminator (which is ``\n`` by
///         default). When this mode is enabled, the behavior changes such that ``.`` truly matches
///         any character.
///
///         This setting can also be configured using the inline flag ``s`` in the pattern. For
///         example, ``(?s:.)`` and ``\p{any}`` are equivalent regexes.
///
///         The default for this is ``False``.
///     crlf: This configures CRLF mode for the entire pattern.
///
///         When CRLF mode is enabled, both ``\r`` ("carriage return" or CR for short) and ``\n``
///         ("line feed" or LF for short) are treated as line terminators. This results in the
///         following:
///
///         * Unless dot-matches-new-line mode is enabled, ``.`` will now match any character except
///           for ``\n`` and ``\r``.
///         * When multi-line mode is enabled, ``^`` will match immediately following a ``\n`` or a
///           ``\r``. Similarly, ``$`` will match immediately preceding a ``\n`` or a ``\r``. Neither
///           ``^`` nor ``$`` will ever match between ``\r`` and ``\n``.
///
///         This setting can also be configured using the inline flag ``R`` in the pattern.
///
///         The default for this is ``False``.
///     line_terminator: Configures the line terminator to be used by the regex. It must be a single
///         ASCII character.
///
///         The line terminator is relevant in two ways for a particular regex:
///
///         * When dot-matches-new-line mode is *not* enabled (the default), then ``.`` will match any
///           character except for the configured line terminator.
///         * When multi-line mode is enabled (not the default), then ``^`` and ``$`` will match
///           immediately after and before, respectively, a line terminator.
///
///         In both cases, if CRLF mode is enabled in a particular context, then it takes precedence
///         over any configured line terminator.
///
///         This option cannot be configured from within the pattern.
///
///         The default line terminator is ``\n``.
///     swap_greed: This configures swap-greed mode for the entire pattern.
///
///         When swap-greed mode is enabled, patterns like ``a+`` will become non-greedy and patterns
///         like ``a+?`` will become greedy. In other words, the meanings of ``a+`` and ``a+?`` are
///         switched.
///
///         This setting can also be configured using the inline flag ``U`` in the pattern.
///
///         The default for this is ``False``.
///     ignore_whitespace: This configures verbose mode for the entire pattern.
///
///         When enabled, whitespace will treated as insignificant in the pattern and ``#`` can be used
///         to start a comment until the next new line.
///
///         Normally, in most places in a pattern, whitespace is treated literally. For example ``" +"``
///         will match one or more ASCII whitespace characters.
///
///         When verbose mode is enabled, ``\#`` can be used to match a literal ``#`` and ``\``
///         followed by a space can be used to match a literal ASCII whitespace character.
///
///         Verbose mode is useful for permitting regexes to be formatted and broken up more nicely.
///         This may make them more easily readable.
///
///         This setting can also be configured using the inline flag ``x`` in the pattern.
///
///         The default for this is ``False``.
///     octal: This configures octal mode for the entire pattern.
///
///         Octal syntax is a little-known way of uttering Unicode codepoints in a pattern. For
///         example, ``a``, ``\x61``, ``\u0061`` and ``\141`` are all equivalent patterns, where the
///         last example shows octal syntax.
///
///         While supporting octal syntax isn't in and of itself a problem, it does make good error
///         messages harder. That is, in PCRE based regex engines, syntax like ``\1`` invokes a
///         backreference, which is explicitly unsupported this library. However, many users expect
///         backreferences to be supported. Therefore, when octal support is disabled, the error
///         message will explicitly mention that backreferences aren't supported.
///
///         The default for this is ``False``.
///     size_limit: Sets the approximate size limit, in bytes, of the compiled regex.
///
///         This roughly corresponds to the number of heap memory, in bytes, occupied by a single
///         regex. If the regex would otherwise approximately exceed this limit, then compiling that
///         regex will fail.
///
///         The main utility of a method like this is to avoid compiling regexes that use an unexpected
///         amount of resources, such as time and memory. Even if the memory usage of a large regex is
///         acceptable, its search time may not be. Namely, worst case time complexity for search is
///         ``O(m * n)``, where ``m ~ len(pattern)`` and ``n ~ len(haystack)``. That is, search time
///         depends, in part, on the size of the compiled regex. This means that putting a limit on the
///         size of the regex limits how much a regex can impact search time.
///
///         For more information about regex size limits, see the section on `untrusted inputs
///         <https://docs.rs/regex/1.13.1/regex/#untrusted-input>`__ in the documentation of the Rust
///         ``regex`` crate.
///
///         The default for this is some reasonable number that permits most patterns to compile
///         successfully.
///     dfa_size_limit: Set the approximate capacity, in bytes, of the cache of transitions used by
///         the lazy DFA.
///
///         While the lazy DFA isn't always used, in tends to be the most commonly use regex engine in
///         default configurations. It tends to adopt the performance profile of a fully build DFA, but
///         without the downside of taking worst case exponential time to build.
///
///         The downside is that it needs to keep a cache of transitions and states that are built
///         while running a search, and this cache can fill up. When it fills up, the cache will reset
///         itself. Any previously generated states and transitions will then need to be re-generated.
///         If this happens too many times, then this library will bail out of using the lazy DFA and
///         switch to a different regex engine.
///
///         If your regex provokes this particular downside of the lazy DFA, then it may be beneficial
///         to increase its cache capacity. This will potentially reduce the frequency of cache
///         resetting (ideally to ``0``). While it won't fix all potential performance problems with
///         the lazy DFA, increasing the cache capacity does fix some.
///
///         There is no easy way to determine, a priori, whether increasing this cache capacity will
///         help. In general, the larger your regex, the more cache it's likely to use. But that isn't
///         an ironclad rule. For example, a regex like ``[01]*1[01]{N}`` would normally produce a
///         fully build DFA that is exponential in size with respect to ``N``. The lazy DFA will
///         prevent exponential space blow-up, but it cache is likely to fill up, even when it's large
///         and even for smallish values of ``N``.
///
///         If you aren't sure whether this helps or not, it is sensible to set this to some
///         arbitrarily large number in testing. Namely, this represents the amount of capacity that
///         *may* be used. It's probably not a good idea to use such a number in production though,
///         since it implies there are no controls on heap memory used by this library during a
///         search. In effect, set it to whatever you're willing to allocate for a single regex search.
///     nest_limit: Set the nesting limit for the parser.
///
///         The nesting limit controls how deep the abstract syntax tree is allowed to be. If the AST
///         exceeds the given limit (e.g., with too many nested groups), then an error is returned by
///         the parser.
///
///         This limit is not checked until the entire AST is parsed. Therefore, if callers want to put
///         a limit on the amount of heap space used, then they should impose a limit on the length,
///         in bytes, of the concrete pattern string. In particular, this is viable since this parser
///         implementation will limit itself to heap space proportional to the length of the pattern
///         string. See also the `untrusted inputs
///         <https://docs.rs/regex/1.13.1/regex/#untrusted-input>`__ section in the documentation of
///         the Rust ``regex`` crate for more information about this.
///
///         Note that a nest limit of ``0`` will return a nest limit error for most patterns but not
///         all. For example, a nest limit of ``0`` permits ``a`` but not ``ab``, since ``ab`` requires
///         an explicit concatenation, which results in a nest depth of ``1``. In general, a nest limit
///         is not something that manifests in an obvious way in the concrete syntax, therefore, it
///         should not be used in a granular way.
///
/// Raises:
///     PatternSyntaxError: If an invalid pattern is given.
///     CompiledTooBigError: If the pattern is valid, but would produce a regex that is bigger than the
///         configured size limit via ``size_limit``. (A reasonable size limit is enabled by default.)
///
/// Examples:
///     Find the offsets of a US phone number:
///
///     >>> regex = Regex("[0-9]{3}-[0-9]{3}-[0-9]{4}")
///     >>> regex.find("phone: 111-222-3333").span
///     (7, 19)
///
///     A common way to use regexes is with capture groups. That is, instead of just looking for
///     matches of an entire regex, parentheses are used to create groups that represent part of the
///     match.
///
///     For example, consider a haystack with multiple lines, and each line has three whitespace
///     delimited fields where the second field is expected to be a number and the third field a
///     boolean. To make this convenient, we use the :meth:`Captures.extract` API to put the strings
///     that match each group into a tuple:
///
///     >>> haystack = "rabbit         54 true\ngroundhog 2 true\ndoes not match\nfox   109    false"
///     >>> regex = Regex(r"(?m)^\s*(\S+)\s+([0-9]+)\s+(true|false)\s*$")
///     >>> fields = []
///     >>> for _, (f1, f2, f3) in (caps.extract() for caps in regex.captures_iter(haystack)):
///     ...     fields.append((f1, int(f2), f3 == "true"))
///     >>> fields
///     [('rabbit', 54, True), ('groundhog', 2, True), ('fox', 109, False)]
///
///     An invalid pattern because of an unclosed parenthesis:
///
///     >>> Regex(r"foo(bar")  # doctest: +IGNORE_EXCEPTION_DETAIL
///     Traceback (most recent call last):
///     PatternSyntaxError: ...
///
///     An invalid pattern because the regex would be too big because Unicode tends to inflate things:
///
///     >>> Regex(r"\w{1000}")  # doctest: +IGNORE_EXCEPTION_DETAIL
///     Traceback (most recent call last):
///     CompiledTooBigError: ...
///
///     Disabling Unicode can make the regex much smaller, potentially by up to or more than an order
///     of magnitude:
///
///     >>> Regex(r"(?-u:\w){1000}")
///     Regex('(?-u:\\w){1000}')
#[pyclass(frozen, module = "regex_rs")]
pub struct Regex {
    pub(crate) inner: regex::Regex,
    options: Options,
}

#[pymethods]
impl Regex {
    #[new]
    #[pyo3(signature = (
        pattern,
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
        pattern: &str,
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
        let mut builder = regex::RegexBuilder::new(pattern);
        let inner = configure!(&mut builder, &options)
            .build()
            .map_err(|err| to_py_err(py, err))?;
        Ok(Self { inner, options })
    }

    /// The original string of this regex.
    ///
    /// Example:
    ///     >>> Regex(r"foo\w+bar").pattern
    ///     'foo\\w+bar'
    #[getter]
    fn pattern(&self) -> &str {
        self.inner.as_str()
    }

    /// The number of capture groups in this regex.
    ///
    /// This includes all named and unnamed groups, including the implicit unnamed group that is always
    /// present and corresponds to the entire match.
    ///
    /// Since the implicit unnamed group is always included in this length, the length is guaranteed to be
    /// greater than zero.
    ///
    /// Example:
    ///     >>> Regex(r"foo").captures_len
    ///     1
    ///     >>> Regex(r"(foo)").captures_len
    ///     2
    ///     >>> Regex(r"(?<a>.(?<b>.))(.)(?:.)(?<c>.)").captures_len
    ///     5
    ///     >>> Regex(r"[a&&b]").captures_len
    ///     1
    #[getter]
    fn captures_len(&self) -> usize {
        self.inner.captures_len()
    }

    /// The total number of capturing groups that appear in every possible match.
    ///
    /// If the number of capture groups can vary depending on the match, then this is ``None``. That is, a
    /// value is only given when the number of matching groups is invariant or "static."
    ///
    /// Note that like :attr:`captures_len`, this **does** include the implicit capturing group
    /// corresponding to the entire match. Therefore, when a non-``None`` value is given, it is guaranteed
    /// to be at least ``1``. Stated differently, a value of ``0`` is impossible.
    ///
    /// Example:
    ///     This shows a few cases where a static number of capture groups is available and a few cases
    ///     where it is not.
    ///
    ///     >>> patterns = ["a", "(a)", "(a)|(b)", "(a)(b)|(c)(d)", "(a)|b", "a|(b)", "(b)*", "(b)+"]
    ///     >>> [Regex(pattern).static_captures_len for pattern in patterns]
    ///     [1, 2, 2, 3, None, None, None, 2]
    #[getter]
    fn static_captures_len(&self) -> Option<usize> {
        self.inner.static_captures_len()
    }

    /// The capture names in this regex.
    ///
    /// The list contains values for all capture groups, even ones that are unnamed (``None``). The order
    /// of the groups corresponds to the order of the group's corresponding opening parenthesis.
    ///
    /// The first element of the list always corresponds to the overall match, and this group is always
    /// unnamed. Therefore, the list always contains at least one group.
    ///
    /// Examples:
    ///     This shows basic usage with a mix of named and unnamed capture groups (the ``(?:.)`` group is
    ///     non-capturing and so doesn't appear here):
    ///
    ///     >>> Regex(r"(?<a>.(?<b>.))(.)(?:.)(?<c>.)").capture_names
    ///     [None, 'a', 'b', None, 'c']
    ///
    ///     The list always contains at least one element, even for regexes with no capture groups and even
    ///     for regexes that can never match:
    ///
    ///     >>> Regex(r"").capture_names
    ///     [None]
    ///     >>> Regex(r"[a&&b]").capture_names
    ///     [None]
    #[getter]
    fn capture_names(&self) -> Vec<Option<&str>> {
        self.inner.capture_names().collect()
    }

    /// Returns ``True`` if and only if there is a match for the regex anywhere in the haystack given.
    ///
    /// It is recommended to use this method if all you need to do is test whether a match exists, since
    /// the underlying matching engine may be able to do less work.
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
    ///     Test if some haystack contains at least one word with exactly 13 Unicode word characters:
    ///
    ///     >>> Regex(r"\b\w{13}\b").is_match("I categorically deny having triskaidekaphobia.")
    ///     True
    ///
    ///     This example shows the significance of ``start`` by demonstrating how it can be used to permit
    ///     look-around assertions in a regex to take the surrounding context into account.
    ///
    ///     >>> regex = Regex(r"\bchew\b")
    ///     >>> # We get a match here, but it's probably not intended.
    ///     >>> regex.is_match("eschew"[2:])
    ///     True
    ///     >>> # No match because the assertions take the context into account.
    ///     >>> regex.is_match("eschew", 2)
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
        let (regex, text) = (&self.inner, haystack.text);
        Ok(search(py, text.len(), || regex.is_match_at(text, start)))
    }

    /// This routine searches for the first match of this regex in the haystack given, and if found,
    /// returns a :class:`Match`. The :class:`Match` provides access to both the offsets of the match and
    /// the actual substring that matched.
    ///
    /// Note that this should only be used if you want to find the entire match. If instead you just want
    /// to test the existence of a match, it's potentially faster to use ``regex.is_match(haystack)``
    /// instead of ``regex.find(haystack) is not None``.
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
    ///     Find the first word with exactly 13 Unicode word characters:
    ///
    ///     >>> m = Regex(r"\b\w{13}\b").find("I categorically deny having triskaidekaphobia.")
    ///     >>> m.span, m.text
    ///     ((2, 15), 'categorically')
    ///
    ///     This example shows the significance of ``start`` by demonstrating how it can be used to permit
    ///     look-around assertions in a regex to take the surrounding context into account.
    ///
    ///     >>> regex = Regex(r"\bchew\b")
    ///     >>> # We get a match here, but it's probably not intended.
    ///     >>> regex.find("eschew"[2:]).span
    ///     (0, 4)
    ///     >>> # No match because the assertions take the context into account.
    ///     >>> regex.find("eschew", 2) is None
    ///     True
    #[pyo3(signature = (haystack, /, start = 0))]
    fn find(
        &self,
        py: Python<'_>,
        haystack: &Bound<'_, PyString>,
        start: usize,
    ) -> PyResult<Option<Match>> {
        let haystack = Haystack::new(haystack)?;
        let byte_start = haystack.byte_offset(start)?;
        let (regex, text) = (&self.inner, haystack.text);
        let found = search(py, text.len(), || regex.find_at(text, byte_start));
        Ok(found.map(|m| {
            let mut cursor = haystack.cursor_at(byte_start, start);
            Match::new(
                py,
                &haystack.obj.clone().unbind(),
                cursor.span(text, m.start(), m.end()),
            )
        }))
    }

    /// Returns an iterator that yields successive non-overlapping matches in the given haystack. The
    /// iterator yields values of type :class:`Match`.
    ///
    /// Time complexity:
    ///     Note that since ``find_iter`` runs potentially many searches on the haystack and since each search
    ///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
    ///     iteration is ``O(m * n^2)``.
    ///
    /// Example:
    ///     Find every word with exactly 13 Unicode word characters:
    ///
    ///     >>> haystack = "Retroactively relinquishing remunerations is reprehensible."
    ///     >>> [m.text for m in Regex(r"\b\w{13}\b").find_iter(haystack)]
    ///     ['Retroactively', 'relinquishing', 'remunerations', 'reprehensible']
    #[pyo3(signature = (haystack, /))]
    fn find_iter(slf: &Bound<'_, Self>, haystack: &Bound<'_, PyString>) -> PyResult<Matches> {
        Ok(Matches::new(
            slf.clone().unbind(),
            &Haystack::new(haystack)?,
        ))
    }

    /// This routine searches for the first match of this regex in the haystack given, and if found,
    /// returns not only the overall match but also the matches of each capture group in the regex. If no
    /// match is found, then ``None`` is returned.
    ///
    /// Capture group ``0`` always corresponds to an implicit unnamed group that includes the entire match.
    /// If a match is found, this group is always present. Subsequent groups may be named and are
    /// numbered, starting at 1, by the order in which the opening parenthesis appears in the pattern. For
    /// example, in the pattern ``(?<a>.(?<b>.))(?<c>.)``, ``a``, ``b`` and ``c`` correspond to capture
    /// group indices ``1``, ``2`` and ``3``, respectively.
    ///
    /// You should only use ``captures`` if you need access to the capture group matches. Otherwise,
    /// :meth:`find` is generally faster for discovering just the overall match.
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
    ///     Say you have some haystack with movie names and their release years, like "'Citizen Kane'
    ///     (1941)". It'd be nice if we could search for substrings looking like that, while also
    ///     extracting the movie name and its release year separately. The example below shows how to do
    ///     that.
    ///
    ///     >>> regex = Regex(r"'([^']+)'\s+\((\d{4})\)")
    ///     >>> caps = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).")
    ///     >>> caps.get(0).text, caps.get(1).text, caps.get(2).text
    ///     ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
    ///     >>> # You can also access the groups by index using the subscript notation.
    ///     >>> caps[0].text, caps[1].text, caps[2].text
    ///     ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
    ///
    ///     Note that the full match is at capture group ``0``. Each subsequent capture group is indexed by
    ///     the order of its opening ``(``.
    ///
    ///     We can make this example a bit clearer by using *named* capture groups:
    ///
    ///     >>> regex = Regex(r"'(?<title>[^']+)'\s+\((?<year>\d{4})\)")
    ///     >>> caps = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).")
    ///     >>> caps.get(0).text, caps.name("title").text, caps.name("year").text
    ///     ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
    ///     >>> # You can also access the groups by name using the subscript notation.
    ///     >>> caps[0].text, caps["title"].text, caps["year"].text
    ///     ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
    ///
    ///     Here we name the capture groups, which we can access with the ``name`` method or the subscript
    ///     notation with a ``str``. Note that the named capture groups are still accessible with ``get``
    ///     or the subscript notation with an ``int``.
    ///
    ///     The capture group at index ``0`` is always unnamed, so it must always be accessed with
    ///     ``get(0)`` or ``[0]``.
    ///
    ///     Finally, one other way to get the matched substrings is with the :meth:`Captures.extract` API:
    ///
    ///     >>> regex = Regex(r"'([^']+)'\s+\((\d{4})\)")
    ///     >>> full, (title, year) = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).").extract()
    ///     >>> full, title, year
    ///     ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
    ///
    ///     This example shows the significance of ``start`` by demonstrating how it can be used to permit
    ///     look-around assertions in a regex to take the surrounding context into account.
    ///
    ///     >>> regex = Regex(r"\bchew\b")
    ///     >>> # We get a match here, but it's probably not intended.
    ///     >>> regex.captures("eschew"[2:])[0].text
    ///     'chew'
    ///     >>> # No match because the assertions take the context into account.
    ///     >>> regex.captures("eschew", 2) is None
    ///     True
    #[pyo3(signature = (haystack, /, start = 0))]
    fn captures(
        slf: &Bound<'_, Self>,
        haystack: &Bound<'_, PyString>,
        start: usize,
    ) -> PyResult<Option<Captures>> {
        let py = slf.py();
        let haystack = Haystack::new(haystack)?;
        let byte_start = haystack.byte_offset(start)?;
        let (regex, text) = (&slf.get().inner, haystack.text);
        let found = search(py, text.len(), || regex.captures_at(text, byte_start));
        Ok(found.map(|caps| {
            let mut cursor = haystack.cursor_at(byte_start, start);
            Captures::new(py, &slf.clone().unbind(), &haystack, &caps, &mut cursor)
        }))
    }

    /// Returns an iterator that yields successive non-overlapping matches in the given haystack. The
    /// iterator yields values of type :class:`Captures`.
    ///
    /// This is the same as :meth:`find_iter`, but instead of only providing access to the overall match,
    /// each value yield includes access to the matches of all capture groups in the regex. Reporting this
    /// extra match data is potentially costly, so callers should only use ``captures_iter`` over
    /// ``find_iter`` when they actually need access to the capture group matches.
    ///
    /// Time complexity:
    ///     Note that since ``captures_iter`` runs potentially many searches on the haystack and since each search
    ///     has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
    ///     iteration is ``O(m * n^2)``.
    ///
    /// Examples:
    ///     We can use this to find all movie titles and their release years in some haystack, where the
    ///     movie is formatted like "'Title' (xxxx)":
    ///
    ///     >>> regex = Regex(r"'([^']+)'\s+\(([0-9]{4})\)")
    ///     >>> haystack = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931)."
    ///     >>> movies = []
    ///     >>> for _, (title, year) in (caps.extract() for caps in regex.captures_iter(haystack)):
    ///     ...     movies.append((title, int(year)))
    ///     >>> movies
    ///     [('Citizen Kane', 1941), ('The Wizard of Oz', 1939), ('M', 1931)]
    ///
    ///     Or with named groups:
    ///
    ///     >>> regex = Regex(r"'(?<title>[^']+)'\s+\((?<year>[0-9]{4})\)")
    ///     >>> it = regex.captures_iter(haystack)
    ///     >>> caps = next(it)
    ///     >>> caps["title"].text, caps["year"].text
    ///     ('Citizen Kane', '1941')
    ///     >>> caps = next(it)
    ///     >>> caps["title"].text, caps["year"].text
    ///     ('The Wizard of Oz', '1939')
    ///     >>> caps = next(it)
    ///     >>> caps["title"].text, caps["year"].text
    ///     ('M', '1931')
    #[pyo3(signature = (haystack, /))]
    fn captures_iter(
        slf: &Bound<'_, Self>,
        haystack: &Bound<'_, PyString>,
    ) -> PyResult<CaptureMatches> {
        Ok(CaptureMatches::new(
            slf.clone().unbind(),
            &Haystack::new(haystack)?,
        ))
    }

    /// Returns an iterator of substrings of the haystack given, delimited by a match of the regex. Namely,
    /// each element of the iterator corresponds to a part of the haystack that *isn't* matched by the
    /// regular expression.
    ///
    /// Time complexity:
    ///     Since iterators over all matches requires running potentially many searches on the haystack,
    ///     and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
    ///     complexity for this routine is ``O(m * n^2)``.
    ///
    /// Examples:
    ///     To split a string delimited by arbitrary amounts of spaces or tabs:
    ///
    ///     >>> list(Regex(r"[ \t]+").split("a b \t  c\td    e"))
    ///     ['a', 'b', 'c', 'd', 'e']
    ///
    ///     Basic usage:
    ///
    ///     >>> list(Regex(r" ").split("Mary had a little lamb"))
    ///     ['Mary', 'had', 'a', 'little', 'lamb']
    ///     >>> list(Regex(r"X").split(""))
    ///     ['']
    ///     >>> list(Regex(r"X").split("lionXXtigerXleopard"))
    ///     ['lion', '', 'tiger', 'leopard']
    ///     >>> list(Regex(r"::").split("lion::tiger::leopard"))
    ///     ['lion', 'tiger', 'leopard']
    ///
    ///     If a haystack contains multiple contiguous matches, you will end up with empty spans yielded by
    ///     the iterator:
    ///
    ///     >>> list(Regex(r"X").split("XXXXaXXbXc"))
    ///     ['', '', '', '', 'a', '', 'b', 'c']
    ///     >>> list(Regex(r"/").split("(///)"))
    ///     ['(', '', '', ')']
    ///
    ///     Separators at the start or end of a haystack are neighbored by empty substring.
    ///
    ///     >>> list(Regex(r"0").split("010"))
    ///     ['', '1', '']
    ///
    ///     When the empty string is used as a regex, it splits at every valid UTF-8 boundary by default
    ///     (which includes the beginning and end of the haystack):
    ///
    ///     >>> list(Regex(r"").split("rust"))
    ///     ['', 'r', 'u', 's', 't', '']
    ///     >>> # Splitting by an empty string is UTF-8 aware by default!
    ///     >>> list(Regex(r"").split("☃"))
    ///     ['', '☃', '']
    ///
    ///     Contiguous separators (commonly shows up with whitespace), can lead to possibly surprising
    ///     behavior. For example, this code is correct:
    ///
    ///     >>> list(Regex(r" ").split("    a  b c"))
    ///     ['', '', '', '', 'a', '', 'b', 'c']
    ///
    ///     It does *not* give you ``['a', 'b', 'c']``. For that behavior, you'd want to match contiguous
    ///     space characters (note that this does still include a leading empty span because ``' +'``
    ///     matches at the beginning of the haystack):
    ///
    ///     >>> list(Regex(r" +").split("    a  b c"))
    ///     ['', 'a', 'b', 'c']
    #[pyo3(signature = (haystack, /))]
    fn split(slf: &Bound<'_, Self>, haystack: &Bound<'_, PyString>) -> PyResult<Split> {
        Ok(Split::new(slf.clone().unbind(), &Haystack::new(haystack)?))
    }

    /// Returns an iterator of at most ``limit`` substrings of the haystack given, delimited by a match of
    /// the regex. (A ``limit`` of ``0`` will return no substrings.) Namely, each element of the iterator
    /// corresponds to a part of the haystack that *isn't* matched by the regular expression. The remainder
    /// of the haystack that is not split will be the last element in the iterator.
    ///
    /// Time complexity:
    ///     Since iterators over all matches requires running potentially many searches on the haystack,
    ///     and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
    ///     complexity for this routine is ``O(m * n^2)``.
    ///
    ///     Although note that the worst case time here has an upper bound given by the ``limit``
    ///     parameter.
    ///
    /// Examples:
    ///     Get the first two words in some haystack:
    ///
    ///     >>> list(Regex(r"\W+").splitn("Hey! How are you?", 3))
    ///     ['Hey', 'How', 'are you?']
    ///
    ///     More cases:
    ///
    ///     >>> list(Regex(r" ").splitn("Mary had a little lamb", 3))
    ///     ['Mary', 'had', 'a little lamb']
    ///     >>> list(Regex(r"X").splitn("", 3))
    ///     ['']
    ///     >>> list(Regex(r"X").splitn("lionXXtigerXleopard", 3))
    ///     ['lion', '', 'tigerXleopard']
    ///     >>> list(Regex(r"::").splitn("lion::tiger::leopard", 2))
    ///     ['lion', 'tiger::leopard']
    ///     >>> list(Regex(r"X").splitn("abcXdef", 1))
    ///     ['abcXdef']
    ///     >>> list(Regex(r"X").splitn("abcdef", 2))
    ///     ['abcdef']
    ///     >>> list(Regex(r"X").splitn("abcXdef", 0))
    ///     []
    #[pyo3(signature = (haystack, limit, /))]
    fn splitn(
        slf: &Bound<'_, Self>,
        haystack: &Bound<'_, PyString>,
        limit: usize,
    ) -> PyResult<SplitN> {
        Ok(SplitN::new(
            slf.clone().unbind(),
            &Haystack::new(haystack)?,
            limit,
        ))
    }

    /// Replaces the leftmost-first match in the given haystack with the replacement provided. The
    /// replacement can be a regular string (where ``$N`` and ``$name`` are expanded to match capture
    /// groups) or a function that takes a :class:`Captures` and returns the replaced string.
    ///
    /// If no match is found, then the haystack is returned unchanged.
    ///
    /// Args:
    ///     haystack: The haystack to search.
    ///     replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
    ///         match capture groups), or a function that takes a :class:`Captures` and returns the
    ///         replaced string.
    ///     literal: Force literal string replacement, without expanding ``$name`` to their
    ///         corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
    ///         example) and faster (since capture groups don't need to be found). Only applies to string
    ///         replacements.
    ///
    /// Raises:
    ///     TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
    ///         a function.
    ///
    /// Replacement string syntax:
    ///     All instances of ``$ref`` in the replacement string are replaced with the substring
    ///     corresponding to the capture group identified by ``ref``.
    ///
    ///     ``ref`` may be an integer corresponding to the index of the capture group (counted by order of
    ///     opening parenthesis where ``0`` is the entire match) or it can be a name (consisting of letters,
    ///     digits or underscores) corresponding to a named capture group.
    ///
    ///     If ``ref`` isn't a valid capture group (whether the name doesn't exist or isn't a valid index),
    ///     then it is replaced with the empty string.
    ///
    ///     The longest possible name is used. For example, ``$1a`` looks up the capture group named ``1a``
    ///     and not the capture group at index ``1``. To exert more precise control over the name, use
    ///     braces, e.g., ``${1}a``.
    ///
    ///     To write a literal ``$`` use ``$$``.
    ///
    /// Examples:
    ///     In typical usage, the replacement can just be a normal string:
    ///
    ///     >>> Regex(r"[^01]+").replace("1078910", "")
    ///     '1010'
    ///
    ///     But a function will work too. It is given the :class:`Captures` corresponding to a match,
    ///     which allows one to access capturing group matches easily:
    ///
    ///     >>> regex = Regex(r"([^,\s]+),\s+(\S+)")
    ///     >>> regex.replace("Springsteen, Bruce", lambda caps: f"{caps[2].text} {caps[1].text}")
    ///     'Bruce Springsteen'
    ///
    ///     But this is a bit cumbersome to use all the time. Instead, a simple syntax is supported (as
    ///     described above) that expands ``$name`` into the corresponding capture group. Here's the last
    ///     example, but using this expansion technique with named capture groups:
    ///
    ///     >>> Regex(r"(?<last>[^,\s]+),\s+(?<first>\S+)").replace("Springsteen, Bruce", "$first $last")
    ///     'Bruce Springsteen'
    ///
    ///     Note that using ``$2`` instead of ``$first`` or ``$1`` instead of ``$last`` would produce the
    ///     same result. To write a literal ``$`` use ``$$``.
    ///
    ///     Sometimes the replacement string requires use of curly braces to delineate a capture group
    ///     replacement when it is adjacent to some other literal text. For example, if we wanted to join
    ///     two words together with an underscore:
    ///
    ///     >>> Regex(r"(?<first>\w+)\s+(?<second>\w+)").replace("deep fried", "${first}_$second")
    ///     'deep_fried'
    ///
    ///     Without the curly braces, the capture group name ``first_`` would be used, and since it doesn't
    ///     exist, it would be replaced with the empty string.
    ///
    ///     Finally, sometimes you just want to replace a literal string with no regard for capturing group
    ///     expansion. This can be done with ``literal=True``:
    ///
    ///     >>> Regex(r"(?<last>[^,\s]+),\s+(\S+)").replace("Springsteen, Bruce", "$2 $last", literal=True)
    ///     '$2 $last'
    ///
    ///     Using ``literal=True`` may also be faster, since the replacement string won't need to be parsed
    ///     for the ``$`` syntax.
    #[pyo3(signature = (
        haystack,
        replacement: "str | collections.abc.Callable[[Captures], str]",
        /,
        *,
        literal = false,
    ) -> "str")]
    fn replace<'py>(
        slf: &Bound<'py, Self>,
        haystack: &Bound<'py, PyString>,
        replacement: &Bound<'py, PyAny>,
        literal: bool,
    ) -> PyResult<Bound<'py, PyString>> {
        replacen(slf, haystack, 1, replacement, literal)
    }

    /// Replaces all non-overlapping matches in the haystack with the replacement provided. This is the
    /// same as calling :meth:`replacen` with ``limit`` set to ``0``.
    ///
    /// If no match is found, then the haystack is returned unchanged.
    ///
    /// The documentation for :meth:`replace` goes into more detail about what kinds of replacement strings
    /// are supported.
    ///
    /// Args:
    ///     haystack: The haystack to search.
    ///     replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
    ///         match capture groups), or a function that takes a :class:`Captures` and returns the
    ///         replaced string.
    ///     literal: Force literal string replacement, without expanding ``$name`` to their
    ///         corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
    ///         example) and faster (since capture groups don't need to be found). Only applies to string
    ///         replacements.
    ///
    /// Raises:
    ///     TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
    ///         a function.
    ///
    /// Time complexity:
    ///     Since iterators over all matches requires running potentially many searches on the haystack,
    ///     and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
    ///     complexity for this routine is ``O(m * n^2)``.
    ///
    /// Fallibility:
    ///     If a replacement function raises an exception, the exception is only raised once the search
    ///     is over, because there's no way to stop the search process if a replacement fails. Instead, if
    ///     you need to stop as soon as a replacement fails, you should consider implementing your own
    ///     replacement routine:
    ///
    ///     >>> def replace_all(regex, haystack, replacement):
    ///     ...     new = []
    ///     ...     last_match = 0
    ///     ...     for caps in regex.captures_iter(haystack):
    ///     ...         m = caps.get_match()
    ///     ...         new.append(haystack[last_match : m.start])
    ///     ...         new.append(replacement(caps))
    ///     ...         last_match = m.end
    ///     ...     new.append(haystack[last_match:])
    ///     ...     return "".join(new)
    ///
    ///     Let's replace each word with the number of characters in that word. But if we see a word that
    ///     is "too long," we'll give up.
    ///
    ///     >>> def replacement(caps):
    ///     ...     if len(caps[0]) >= 5:
    ///     ...         raise ValueError("word too long")
    ///     ...     return str(len(caps[0]))
    ///     >>> replace_all(Regex(r"\w+"), "hi how are you?", replacement)
    ///     '2 3 3 3?'
    ///     >>> replace_all(Regex(r"\w+"), "hi there", replacement)
    ///     Traceback (most recent call last):
    ///     ValueError: word too long
    ///
    /// Example:
    ///     This example shows how to flip the order of whitespace (excluding line terminators) delimited
    ///     fields, and normalizes the whitespace that delimits the fields:
    ///
    ///     >>> haystack = (
    ///     ...     "Greetings  1973\nWild\t1973\nBornToRun\t\t\t\t1975\n"
    ///     ...     "Darkness                    1978\nTheRiver 1980"
    ///     ... )
    ///     >>> regex = Regex(r"(?m)^(\S+)[\s--\r\n]+(\S+)$")
    ///     >>> print(regex.replace_all(haystack, "$2 $1"))
    ///     1973 Greetings
    ///     1973 Wild
    ///     1975 BornToRun
    ///     1978 Darkness
    ///     1980 TheRiver
    #[pyo3(signature = (
        haystack,
        replacement: "str | collections.abc.Callable[[Captures], str]",
        /,
        *,
        literal = false,
    ) -> "str")]
    fn replace_all<'py>(
        slf: &Bound<'py, Self>,
        haystack: &Bound<'py, PyString>,
        replacement: &Bound<'py, PyAny>,
        literal: bool,
    ) -> PyResult<Bound<'py, PyString>> {
        replacen(slf, haystack, 0, replacement, literal)
    }

    /// Replaces at most ``limit`` non-overlapping matches in the haystack with the replacement provided.
    /// If ``limit`` is ``0``, then all non-overlapping matches are replaced. That is,
    /// ``regex.replace_all(haystack, replacement)`` is equivalent to
    /// ``regex.replacen(haystack, 0, replacement)``.
    ///
    /// If no match is found, then the haystack is returned unchanged.
    ///
    /// The documentation for :meth:`replace` goes into more detail about what kinds of replacement strings
    /// are supported.
    ///
    /// Args:
    ///     haystack: The haystack to search.
    ///     limit: The maximum number of matches to replace.
    ///     replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
    ///         match capture groups), or a function that takes a :class:`Captures` and returns the
    ///         replaced string.
    ///     literal: Force literal string replacement, without expanding ``$name`` to their
    ///         corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
    ///         example) and faster (since capture groups don't need to be found). Only applies to string
    ///         replacements.
    ///
    /// Raises:
    ///     TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
    ///         a function.
    ///
    /// Time complexity:
    ///     Since iterators over all matches requires running potentially many searches on the haystack,
    ///     and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
    ///     complexity for this routine is ``O(m * n^2)``.
    ///
    ///     Although note that the worst case time here has an upper bound given by the ``limit``
    ///     parameter.
    ///
    /// Fallibility:
    ///     See the corresponding section in the docs for :meth:`replace_all` for tips on how to deal with
    ///     a replacement routine that can fail.
    ///
    /// Example:
    ///     This example shows how to flip the order of whitespace (excluding line terminators) delimited
    ///     fields, and normalizes the whitespace that delimits the fields. But we only do it for the first
    ///     two matches.
    ///
    ///     >>> haystack = (
    ///     ...     "Greetings  1973\nWild\t1973\nBornToRun\t\t\t\t1975\n"
    ///     ...     "Darkness                    1978\nTheRiver 1980"
    ///     ... )
    ///     >>> regex = Regex(r"(?m)^(\S+)[\s--\r\n]+(\S+)$")
    ///     >>> print(regex.replacen(haystack, 2, "$2 $1"))  # doctest: +NORMALIZE_WHITESPACE
    ///     1973 Greetings
    ///     1973 Wild
    ///     BornToRun 1975
    ///     Darkness 1978
    ///     TheRiver 1980
    #[pyo3(signature = (
        haystack,
        limit,
        replacement: "str | collections.abc.Callable[[Captures], str]",
        /,
        *,
        literal = false,
    ) -> "str")]
    fn replacen<'py>(
        slf: &Bound<'py, Self>,
        haystack: &Bound<'py, PyString>,
        limit: usize,
        replacement: &Bound<'py, PyAny>,
        literal: bool,
    ) -> PyResult<Bound<'py, PyString>> {
        replacen(slf, haystack, limit, replacement, literal)
    }

    /// Returns the end offset of the first match in the haystack given.
    ///
    /// This method may have the same performance characteristics as :meth:`is_match`. Behaviorally, it
    /// doesn't just report whether it match occurs, but also the end offset for a match. In particular,
    /// the offset returned *may be shorter* than the proper end of the leftmost-first match that you would
    /// find via :meth:`find`.
    ///
    /// Note that it is not guaranteed that this routine finds the shortest or "earliest" possible match.
    /// Instead, the main idea of this API is that it returns the offset at the point at which the internal
    /// regex engine has determined that a match has occurred. This may vary depending on which internal
    /// regex engine is used, and thus, the offset itself may change based on internal heuristics.
    ///
    /// If a match is found, the offset returned is relative to the beginning of the haystack, not the
    /// beginning of the search.
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
    ///     Typically, ``a+`` would match the entire first sequence of ``a`` in some haystack, but
    ///     ``shortest_match`` *may* give up as soon as it sees the first ``a``.
    ///
    ///     >>> Regex(r"a+").shortest_match("aaaaa")
    ///     1
    ///
    ///     This example shows the significance of ``start`` by demonstrating how it can be used to permit
    ///     look-around assertions in a regex to take the surrounding context into account.
    ///
    ///     >>> regex = Regex(r"\bchew\b")
    ///     >>> # We get a match here, but it's probably not intended.
    ///     >>> regex.shortest_match("eschew"[2:])
    ///     4
    ///     >>> # No match because the assertions take the context into account.
    ///     >>> regex.shortest_match("eschew", 2) is None
    ///     True
    #[pyo3(signature = (haystack, /, start = 0))]
    fn shortest_match(
        &self,
        py: Python<'_>,
        haystack: &Bound<'_, PyString>,
        start: usize,
    ) -> PyResult<Option<usize>> {
        let haystack = Haystack::new(haystack)?;
        let byte_start = haystack.byte_offset(start)?;
        let (regex, text) = (&self.inner, haystack.text);
        let found = search(py, text.len(), || regex.shortest_match_at(text, byte_start));
        Ok(found.map(|end| haystack.cursor_at(byte_start, start).chars(text, end)))
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let pattern = PyString::new(py, self.inner.as_str()).repr()?;
        Ok(format!("Regex({pattern}{})", self.options.repr_kwargs(py)?))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner.as_str() == other.inner.as_str() && self.options == other.options
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        (self.inner.as_str(), &self.options).hash(&mut hasher);
        hasher.finish()
    }

    #[pyo3(signature = () -> "tuple[tuple[str], dict[str, object]]")]
    fn __getnewargs_ex__<'py>(&self, py: Python<'py>) -> PyResult<((&str,), Bound<'py, PyDict>)> {
        Ok(((self.inner.as_str(),), self.options.to_kwargs(py, false)?))
    }

    #[pyo3(signature = () -> "Regex")]
    fn __copy__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (memo, /) -> "Regex")]
    fn __deepcopy__<'py>(slf: PyRef<'py, Self>, memo: &Bound<'py, PyAny>) -> PyRef<'py, Self> {
        let _ = memo;
        slf
    }
}

/// Implements `Regex.replacen()`, used by all the replacement methods.
fn replacen<'py>(
    slf: &Bound<'py, Regex>,
    haystack: &Bound<'py, PyString>,
    limit: usize,
    replacement: &Bound<'py, PyAny>,
    literal: bool,
) -> PyResult<Bound<'py, PyString>> {
    let py = slf.py();
    let regex = &slf.get().inner;
    let hay = Haystack::new(haystack)?;
    let text = hay.text;
    let replaced = if let Ok(replacement) = replacement.cast::<PyString>() {
        let replacement = replacement.to_str()?;
        if literal {
            search(py, text.len(), || {
                regex.replacen(text, limit, regex::NoExpand(replacement))
            })
        } else {
            search(py, text.len(), || regex.replacen(text, limit, replacement))
        }
    } else if replacement.is_callable() {
        if literal {
            return Err(PyTypeError::new_err(
                "literal=True requires a string replacement",
            ));
        }
        let slf_py = slf.clone().unbind();
        let mut cursor = hay.cursor_at(0, 0);
        // The search can't be interrupted, so the first error is kept and raised once it is over.
        let error = RefCell::new(None);
        let replaced = regex.replacen(text, limit, |caps: &regex::Captures<'_>| -> String {
            if error.borrow().is_some() {
                return String::new();
            }
            let captures = Captures::new(py, &slf_py, &hay, caps, &mut cursor);
            match replacement
                .call1((captures,))
                .and_then(|result| result.extract::<String>())
            {
                Ok(result) => result,
                Err(err) => {
                    *error.borrow_mut() = Some(err);
                    String::new()
                }
            }
        });
        if let Some(err) = error.into_inner() {
            return Err(err);
        }
        replaced
    } else {
        return Err(PyTypeError::new_err(format!(
            "replacement must be a string or a callable, not {}",
            replacement.get_type().name()?
        )));
    };
    Ok(match replaced {
        Cow::Borrowed(_) => haystack.clone(),
        Cow::Owned(replaced) => PyString::new(py, &replaced),
    })
}
