import collections.abc
from typing import Any, Final, SupportsIndex, final

REGEX_VERSION: Final[str]
"""
The version of the Rust ``regex`` crate this module was built against.
"""

@final
class CaptureMatches:
    """
    An iterator over all non-overlapping capture matches in a haystack.

    This iterator yields :class:`Captures` values. The iterator stops when no more matches can be
    found.

    This iterator is created by :meth:`Regex.captures_iter`.

    Time complexity:
        Note that since an iterator runs potentially many searches on the haystack and since each search
        has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
        iteration is ``O(m * n^2)``.
    """
    def __iter__(self, /) -> CaptureMatches: ...
    def __next__(self, /) -> Captures: ...

@final
class Captures:
    r"""
    Represents the capture groups for a single match.

    Capture groups refer to parts of a regex enclosed in parentheses. They can be optionally named. The
    purpose of capture groups is to be able to reference different parts of a match based on the
    original pattern. In essence, a :class:`Captures` is a container of :class:`Match` values for each
    group that participated in a regex match. Each :class:`Match` can be looked up by either its
    capture group index or name (if it has one).

    For example, say you want to match the individual letters in a 5-letter word:

    .. code-block:: regex

        (?<first>\w)(\w)(?:\w)\w(?<last>\w)

    This regex has 4 capture groups:

    * The group at index ``0`` corresponds to the overall match. It is always present in every match
      and never has a name.
    * The group at index ``1`` with name ``first`` corresponding to the first letter.
    * The group at index ``2`` with no name corresponding to the second letter.
    * The group at index ``3`` with name ``last`` corresponding to the fifth and last letter.

    Notice that ``(?:\w)`` was not listed above as a capture group despite it being enclosed in
    parentheses. That's because ``(?:pattern)`` is a special syntax that permits grouping but *without*
    capturing. The reason for not treating it as a capture is that tracking and reporting capture
    groups requires additional state that may lead to slower searches. So using as few capture groups
    as possible can help performance. (Although the difference in performance of a couple of capture
    groups is likely immaterial.)

    Values with this type are created by :meth:`Regex.captures` or :meth:`Regex.captures_iter`.

    Example:
        >>> caps = Regex(r"(?<first>\w)(\w)(?:\w)\w(?<last>\w)").captures("toady")
        >>> caps[0].text, caps["first"].text, caps[2].text, caps["last"].text
        ('toady', 't', 'o', 'y')
    """
    def __getitem__(self, key: SupportsIndex | str, /) -> Match | None:
        """
        Get a matching capture group by index or name.

        Unlike :meth:`get` and :meth:`name`, an error is raised for a group that doesn't exist. ``None`` is
        still returned for a group that didn't participate in the match.

        Raises:
            IndexError: If there is no group at the given index.
            KeyError: If there is no group with the given name.
        """
    def __iter__(self, /) -> SubCaptureMatches:
        r"""
        Returns an iterator over all capture groups. This includes both matching and non-matching groups.

        The iterator always yields at least one matching group: the first group (at index ``0``) with no
        name. Subsequent groups are returned in the order of their opening parenthesis in the regex.

        The elements yielded have type ``Match | None``, where a non-``None`` value is present if the
        capture group matches.

        Example:
            >>> caps = Regex(r"(\w)(\d)?(\w)").captures("AZ")
            >>> [m.text if m else None for m in caps]
            ['AZ', 'A', None, 'Z']
        """
    def __len__(self, /) -> int:
        r"""
        Returns the total number of capture groups. This includes both matching and non-matching groups.

        The length returned is always equivalent to the number of elements yielded when iterating over
        this :class:`Captures`. Consequently, the length is always greater than zero since every
        :class:`Captures` value always includes the match for the entire regex.

        Example:
            >>> len(Regex(r"(\w)(\d)?(\w)").captures("AZ"))
            4
        """
    def __repr__(self, /) -> str: ...
    def expand(self, /, replacement: str) -> str:
        """
        Expands all instances of ``$ref`` in ``replacement`` to the corresponding capture group, and
        returns the result. A ``ref`` can be a capture group index or a name. If ``ref`` doesn't refer to a
        capture group that participated in the match, then it is replaced with the empty string.

        Format:
            The format of the replacement string supports two different kinds of capture references:
            unbraced and braced.

            For the unbraced format, the format supported is ``$ref`` where ``name`` can be any character
            in the class ``[0-9A-Za-z_]``. ``ref`` is always the longest possible parse. So for example,
            ``$1a`` corresponds to the capture group named ``1a`` and not the capture group at index ``1``.
            If ``ref`` matches ``^[0-9]+$``, then it is treated as a capture group index itself and not a
            name.

            For the braced format, the format supported is ``${ref}`` where ``ref`` can be any sequence of
            characters except for ``}``. If no closing brace occurs, then it is not considered a capture
            reference. As with the unbraced format, if ``ref`` matches ``^[0-9]+$``, then it is treated as
            a capture group index and not a name.

            The braced format is useful for exerting precise control over the name of the capture
            reference. For example, ``${1}a`` corresponds to the capture group reference ``1`` followed by
            the letter ``a``, where as ``$1a`` (as mentioned above) corresponds to the capture group
            reference ``1a``. The braced format is also useful for expressing capture group names that use
            characters not supported by the unbraced format. For example, ``${foo[bar].baz}`` refers to the
            capture group named ``foo[bar].baz``.

            If a capture group reference is found and it does not refer to a valid capture group, then it
            will be replaced with the empty string.

            To write a literal ``$``, use ``$$``.

        Example:
            >>> regex = Regex(r"(?<day>[0-9]{2})-(?<month>[0-9]{2})-(?<year>[0-9]{4})")
            >>> caps = regex.captures("On 14-03-2010, I became a Tennessee lamb.")
            >>> caps.expand("year=$year, month=$month, day=$day")
            'year=2010, month=03, day=14'
        """
    def extract(self, /) -> tuple[str, tuple[str, ...]]:
        """
        This is a convenience routine for extracting the substrings corresponding to matching capture
        groups.

        This returns a tuple where the first element corresponds to the full substring of the haystack
        that matched the regex. The second element is a tuple of substrings, with each corresponding to the
        substring that matched for a particular capture group.

        This only works when the number of possible matching groups is fixed in all circumstances. More
        precisely, this routine only works when :attr:`Regex.static_captures_len` is not ``None``.

        Stated more plainly, if the number of matching capture groups in a regex can vary from match to
        match, then this function always raises an error.

        For example, ``(a)(b)|(c)`` could produce two matching capture groups or one matching capture group
        for any given match. Therefore, one cannot use ``extract`` with such a pattern.

        But a pattern like ``(a)(b)|(c)(d)`` can be used with ``extract`` because the number of capture
        groups in every match is always equivalent, even if the capture *indices* in each match are not.

        Raises:
            ValueError: If the number of matching capture groups can vary from match to match.

        Examples:
            >>> regex = Regex(r"([0-9]{4})-([0-9]{2})-([0-9]{2})")
            >>> full, (year, month, day) = regex.captures("On 2010-03-14, I became a Tennessee lamb.").extract()
            >>> full, year, month, day
            ('2010-03-14', '2010', '03', '14')

            This example shows how to use this method when iterating over all :class:`Captures` matches in
            a haystack.

            >>> haystack = "1973-01-05, 1975-08-25 and 1980-10-18"
            >>> [groups for _, groups in (caps.extract() for caps in regex.captures_iter(haystack))]
            [('1973', '01', '05'), ('1975', '08', '25'), ('1980', '10', '18')]

            This API is particularly useful when you need to extract a particular value that might occur in
            a different format. Consider, for example, an identifier that might be in double quotes or
            single quotes:

            >>> regex = Regex('id:(?:"([^"]+)"|' + "'([^']+)')")
            >>> haystack = 'The first is id:"foo" and the second is ' + "id:'bar'."
            >>> [id_ for _, (id_,) in (caps.extract() for caps in regex.captures_iter(haystack))]
            ['foo', 'bar']
        """
    def get(self, /, index: SupportsIndex) -> Match | None:
        """
        Returns the :class:`Match` associated with the capture group at index ``index``. If ``index`` does
        not correspond to a capture group, or if the capture group did not participate in the match, then
        ``None`` is returned.

        When ``index == 0``, this is guaranteed to return a non-``None`` value.

        Example:
            Get the substring that matched with a default of an empty string if the group didn't
            participate in the match:

            >>> caps = Regex(r"[a-z]+(?:([0-9]+)|([A-Z]+))").captures("abc123")
            >>> m1, m2 = caps.get(1), caps.get(2)
            >>> m1.text if m1 else "", m2.text if m2 else ""
            ('123', '')
        """
    def get_match(self, /) -> Match:
        """
        Return the overall match for the capture.

        This returns the match for index ``0``. That is it is equivalent to ``caps.get(0)``, except that
        the return value is never ``None``.

        Example:
            >>> Regex(r"[a-z]+([0-9]+)").captures("   abc123-def").get_match().text
            'abc123'
        """
    def name(self, /, name: str) -> Match | None:
        """
        Returns the :class:`Match` associated with the capture group named ``name``. If ``name`` isn't a
        valid capture group or it refers to a group that didn't match, then ``None`` is returned.

        Example:
            Get the substring that matched with a default of an empty string if the group didn't
            participate in the match:

            >>> caps = Regex(r"[a-z]+(?:(?<numbers>[0-9]+)|(?<letters>[A-Z]+))").captures("abc123")
            >>> numbers, letters = caps.name("numbers"), caps.name("letters")
            >>> numbers.text if numbers else "", letters.text if letters else ""
            ('123', '')
        """

@final
class Match:
    r"""
    Represents a single match of a regex in a haystack.

    A :class:`Match` contains both the start and end offsets of the match and the actual substring
    corresponding to the range of those offsets. It is guaranteed that ``start <= end``. When
    ``start == end``, the match is empty.

    Values with this type are created by :meth:`Regex.find` or :meth:`Regex.find_iter`. Other APIs can
    create :class:`Match` values too. For example, :meth:`Captures.get`.

    Numbering:
        The offsets in a :class:`Match` form a half-open interval. That is, the start of the range is
        inclusive and the end of the range is exclusive. For example, given a haystack ``abcFOOxyz``
        and a match of ``FOO``, its offset range starts at ``3`` and ends at ``6``. ``3`` corresponds
        to ``F`` and ``6`` corresponds to ``x``, which is one past the end of the match. This
        corresponds to the same kind of slicing that Python uses.

    Example:
        This example shows the value of each of the attributes and methods on :class:`Match` for a
        particular search.

        >>> m = Regex(r"\p{Greek}+").find("Greek: αβγδ")
        >>> m.start, m.end, m.is_empty(), len(m), m.span, m.text
        (7, 11, False, 4, (7, 11), 'αβγδ')
    """
    def __len__(self, /) -> int:
        """
        The length, in characters, of this match.
        """
    def __repr__(self, /) -> str: ...
    @property
    def end(self, /) -> int:
        """
        The offset of the end of the match in the haystack. The end of the match corresponds to the
        character immediately following the last character in the match. This means that
        ``haystack[start:end]`` works as one would expect.

        It is guaranteed that ``start <= end``.
        """
    def is_empty(self, /) -> bool:
        """
        Returns ``True`` if and only if this match has a length of zero.

        Note that an empty match can only occur when the regex itself can match the empty string. Here are
        some examples of regexes that can all match the empty string: ``^``, ``^$``, ``\b``, ``a?``,
        ``a*``, ``a{0}``, ``(foo|\\d+|quux)?``.
        """
    @property
    def span(self, /) -> tuple[int, int]:
        """
        The starting and ending offsets of the match in the haystack.

        It is always correct to slice the original haystack searched with these offsets.
        """
    @property
    def start(self, /) -> int:
        """
        The offset of the start of the match in the haystack. The start of the match corresponds to the
        position where the match begins and includes the first character in the match.

        It is guaranteed that ``start <= end``.
        """
    @property
    def text(self, /) -> str:
        """
        The substring of the haystack that matched.
        """

@final
class Matches:
    """
    An iterator over all non-overlapping matches in a haystack.

    This iterator yields :class:`Match` values. The iterator stops when no more matches can be found.

    This iterator is created by :meth:`Regex.find_iter`.

    Time complexity:
        Note that since an iterator runs potentially many searches on the haystack and since each search
        has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
        iteration is ``O(m * n^2)``.
    """
    def __iter__(self, /) -> Matches: ...
    def __next__(self, /) -> Match: ...

@final
class Regex:
    """
    A compiled regular expression for searching Unicode haystacks.

    A :class:`Regex` can be used to search haystacks, split haystacks into substrings or replace
    substrings in a haystack with a different substring. All searching is done with an implicit
    ``(?s:.)*?`` at the beginning and end of a pattern. To force an expression to match the whole
    string (or a prefix or a suffix), you must use an anchor like ``^`` or ``$`` (or ``\\A`` and
    ``\\z``).

    While this library will handle Unicode strings (whether in the regular expression or in the
    haystack), all positions returned are offsets of characters in the haystack, which can be used to
    slice it.

    Once compiled, a regular expression can be used repeatedly to search, split or replace substrings in
    a haystack.

    Note that regex compilation tends to be a somewhat expensive process, and unlike higher level
    environments, compilation is not automatically cached for you. One should endeavor to compile a
    regex once and then reuse it. For example, it's a bad idea to compile the same regex repeatedly in
    a loop.

    Args:
        pattern: The regular expression to compile.
        unicode: This configures Unicode mode for the entire pattern.

            Enabling Unicode mode does a number of things:

            * Most fundamentally, it causes the fundamental atom of matching to be a single codepoint.
              When Unicode mode is disabled, it's a single byte. For example, when Unicode mode is
              enabled, ``.`` will match ``💩`` once, where as it will match 4 times when Unicode mode
              is disabled. (Since the UTF-8 encoding of ``💩`` is 4 bytes long.)
            * Case insensitive matching uses Unicode simple case folding rules.
            * Unicode character classes like ``\\p{Letter}`` and ``\\p{Greek}`` are available.
            * Perl character classes are Unicode aware. That is, ``\\w``, ``\\s`` and ``\\d``.
            * The word boundary assertions, ``\b`` and ``\\B``, use the Unicode definition of a word
              character.

            Note that if Unicode mode is disabled, then the regex will fail to compile if it could
            match invalid UTF-8. For example, when Unicode mode is disabled, then since ``.`` matches
            any byte (except for ``\n``), then it can match invalid UTF-8 and thus building a regex
            from it will fail. Another example is ``\\w`` and ``\\W``. Since ``\\w`` can only match ASCII
            bytes when Unicode mode is disabled, it's allowed. But ``\\W`` can match more than ASCII
            bytes, including invalid UTF-8, and so it is not allowed.

            For more details on the Unicode support, see the `Unicode section
            <https://docs.rs/regex/1.13.1/regex/#unicode>`__ of the documentation of the Rust
            ``regex`` crate.

            The default for this is ``True``.
        case_insensitive: This configures whether to enable case insensitive matching for the entire
            pattern.

            This setting can also be configured using the inline flag ``i`` in the pattern. For
            example, ``(?i:foo)`` matches ``foo`` case insensitively while ``(?-i:foo)`` matches
            ``foo`` case sensitively.

            The default for this is ``False``.
        multi_line: This configures multi-line mode for the entire pattern.

            Enabling multi-line mode changes the behavior of the ``^`` and ``$`` anchor assertions.
            Instead of only matching at the beginning and end of a haystack, respectively, multi-line
            mode causes them to match at the beginning and end of a line *in addition* to the
            beginning and end of a haystack. More precisely, ``^`` will match at the position
            immediately following a ``\n`` and ``$`` will match at the position immediately preceding
            a ``\n``.

            The behavior of this option can be impacted by other settings too:

            * The ``line_terminator`` option changes ``\n`` above to any ASCII character.
            * The ``crlf`` option changes the line terminator to be either ``\r`` or ``\n``, but never
              at the position between a ``\r`` and ``\n``.

            This setting can also be configured using the inline flag ``m`` in the pattern.

            The default for this is ``False``.
        dot_matches_new_line: This configures dot-matches-new-line mode for the entire pattern.

            Perhaps surprisingly, the default behavior for ``.`` is not to match any character, but
            rather, to match any character except for the line terminator (which is ``\n`` by
            default). When this mode is enabled, the behavior changes such that ``.`` truly matches
            any character.

            This setting can also be configured using the inline flag ``s`` in the pattern. For
            example, ``(?s:.)`` and ``\\p{any}`` are equivalent regexes.

            The default for this is ``False``.
        crlf: This configures CRLF mode for the entire pattern.

            When CRLF mode is enabled, both ``\r`` ("carriage return" or CR for short) and ``\n``
            ("line feed" or LF for short) are treated as line terminators. This results in the
            following:

            * Unless dot-matches-new-line mode is enabled, ``.`` will now match any character except
              for ``\n`` and ``\r``.
            * When multi-line mode is enabled, ``^`` will match immediately following a ``\n`` or a
              ``\r``. Similarly, ``$`` will match immediately preceding a ``\n`` or a ``\r``. Neither
              ``^`` nor ``$`` will ever match between ``\r`` and ``\n``.

            This setting can also be configured using the inline flag ``R`` in the pattern.

            The default for this is ``False``.
        line_terminator: Configures the line terminator to be used by the regex. It must be a single
            ASCII character.

            The line terminator is relevant in two ways for a particular regex:

            * When dot-matches-new-line mode is *not* enabled (the default), then ``.`` will match any
              character except for the configured line terminator.
            * When multi-line mode is enabled (not the default), then ``^`` and ``$`` will match
              immediately after and before, respectively, a line terminator.

            In both cases, if CRLF mode is enabled in a particular context, then it takes precedence
            over any configured line terminator.

            This option cannot be configured from within the pattern.

            The default line terminator is ``\n``.
        swap_greed: This configures swap-greed mode for the entire pattern.

            When swap-greed mode is enabled, patterns like ``a+`` will become non-greedy and patterns
            like ``a+?`` will become greedy. In other words, the meanings of ``a+`` and ``a+?`` are
            switched.

            This setting can also be configured using the inline flag ``U`` in the pattern.

            The default for this is ``False``.
        ignore_whitespace: This configures verbose mode for the entire pattern.

            When enabled, whitespace will treated as insignificant in the pattern and ``#`` can be used
            to start a comment until the next new line.

            Normally, in most places in a pattern, whitespace is treated literally. For example ``" +"``
            will match one or more ASCII whitespace characters.

            When verbose mode is enabled, ``\\#`` can be used to match a literal ``#`` and ``\\``
            followed by a space can be used to match a literal ASCII whitespace character.

            Verbose mode is useful for permitting regexes to be formatted and broken up more nicely.
            This may make them more easily readable.

            This setting can also be configured using the inline flag ``x`` in the pattern.

            The default for this is ``False``.
        octal: This configures octal mode for the entire pattern.

            Octal syntax is a little-known way of uttering Unicode codepoints in a pattern. For
            example, ``a``, ``\x61``, ``\u0061`` and ``\141`` are all equivalent patterns, where the
            last example shows octal syntax.

            While supporting octal syntax isn't in and of itself a problem, it does make good error
            messages harder. That is, in PCRE based regex engines, syntax like ``\1`` invokes a
            backreference, which is explicitly unsupported this library. However, many users expect
            backreferences to be supported. Therefore, when octal support is disabled, the error
            message will explicitly mention that backreferences aren't supported.

            The default for this is ``False``.
        size_limit: Sets the approximate size limit, in bytes, of the compiled regex.

            This roughly corresponds to the number of heap memory, in bytes, occupied by a single
            regex. If the regex would otherwise approximately exceed this limit, then compiling that
            regex will fail.

            The main utility of a method like this is to avoid compiling regexes that use an unexpected
            amount of resources, such as time and memory. Even if the memory usage of a large regex is
            acceptable, its search time may not be. Namely, worst case time complexity for search is
            ``O(m * n)``, where ``m ~ len(pattern)`` and ``n ~ len(haystack)``. That is, search time
            depends, in part, on the size of the compiled regex. This means that putting a limit on the
            size of the regex limits how much a regex can impact search time.

            For more information about regex size limits, see the section on `untrusted inputs
            <https://docs.rs/regex/1.13.1/regex/#untrusted-input>`__ in the documentation of the Rust
            ``regex`` crate.

            The default for this is some reasonable number that permits most patterns to compile
            successfully.
        dfa_size_limit: Set the approximate capacity, in bytes, of the cache of transitions used by
            the lazy DFA.

            While the lazy DFA isn't always used, in tends to be the most commonly use regex engine in
            default configurations. It tends to adopt the performance profile of a fully build DFA, but
            without the downside of taking worst case exponential time to build.

            The downside is that it needs to keep a cache of transitions and states that are built
            while running a search, and this cache can fill up. When it fills up, the cache will reset
            itself. Any previously generated states and transitions will then need to be re-generated.
            If this happens too many times, then this library will bail out of using the lazy DFA and
            switch to a different regex engine.

            If your regex provokes this particular downside of the lazy DFA, then it may be beneficial
            to increase its cache capacity. This will potentially reduce the frequency of cache
            resetting (ideally to ``0``). While it won't fix all potential performance problems with
            the lazy DFA, increasing the cache capacity does fix some.

            There is no easy way to determine, a priori, whether increasing this cache capacity will
            help. In general, the larger your regex, the more cache it's likely to use. But that isn't
            an ironclad rule. For example, a regex like ``[01]*1[01]{N}`` would normally produce a
            fully build DFA that is exponential in size with respect to ``N``. The lazy DFA will
            prevent exponential space blow-up, but it cache is likely to fill up, even when it's large
            and even for smallish values of ``N``.

            If you aren't sure whether this helps or not, it is sensible to set this to some
            arbitrarily large number in testing. Namely, this represents the amount of capacity that
            *may* be used. It's probably not a good idea to use such a number in production though,
            since it implies there are no controls on heap memory used by this library during a
            search. In effect, set it to whatever you're willing to allocate for a single regex search.
        nest_limit: Set the nesting limit for the parser.

            The nesting limit controls how deep the abstract syntax tree is allowed to be. If the AST
            exceeds the given limit (e.g., with too many nested groups), then an error is returned by
            the parser.

            This limit is not checked until the entire AST is parsed. Therefore, if callers want to put
            a limit on the amount of heap space used, then they should impose a limit on the length,
            in bytes, of the concrete pattern string. In particular, this is viable since this parser
            implementation will limit itself to heap space proportional to the length of the pattern
            string. See also the `untrusted inputs
            <https://docs.rs/regex/1.13.1/regex/#untrusted-input>`__ section in the documentation of
            the Rust ``regex`` crate for more information about this.

            Note that a nest limit of ``0`` will return a nest limit error for most patterns but not
            all. For example, a nest limit of ``0`` permits ``a`` but not ``ab``, since ``ab`` requires
            an explicit concatenation, which results in a nest depth of ``1``. In general, a nest limit
            is not something that manifests in an obvious way in the concrete syntax, therefore, it
            should not be used in a granular way.

    Raises:
        PatternSyntaxError: If an invalid pattern is given.
        CompiledTooBigError: If the pattern is valid, but would produce a regex that is bigger than the
            configured size limit via ``size_limit``. (A reasonable size limit is enabled by default.)

    Examples:
        Find the offsets of a US phone number:

        >>> regex = Regex("[0-9]{3}-[0-9]{3}-[0-9]{4}")
        >>> regex.find("phone: 111-222-3333").span
        (7, 19)

        A common way to use regexes is with capture groups. That is, instead of just looking for
        matches of an entire regex, parentheses are used to create groups that represent part of the
        match.

        For example, consider a haystack with multiple lines, and each line has three whitespace
        delimited fields where the second field is expected to be a number and the third field a
        boolean. To make this convenient, we use the :meth:`Captures.extract` API to put the strings
        that match each group into a tuple:

        >>> haystack = "rabbit         54 true\ngroundhog 2 true\ndoes not match\nfox   109    false"
        >>> regex = Regex(r"(?m)^\\s*(\\S+)\\s+([0-9]+)\\s+(true|false)\\s*$")
        >>> fields = []
        >>> for _, (f1, f2, f3) in (caps.extract() for caps in regex.captures_iter(haystack)):
        ...     fields.append((f1, int(f2), f3 == "true"))
        >>> fields
        [('rabbit', 54, True), ('groundhog', 2, True), ('fox', 109, False)]

        An invalid pattern because of an unclosed parenthesis:

        >>> Regex(r"foo(bar")  # doctest: +IGNORE_EXCEPTION_DETAIL
        Traceback (most recent call last):
        PatternSyntaxError: ...

        An invalid pattern because the regex would be too big because Unicode tends to inflate things:

        >>> Regex(r"\\w{1000}")  # doctest: +IGNORE_EXCEPTION_DETAIL
        Traceback (most recent call last):
        CompiledTooBigError: ...

        Disabling Unicode can make the regex much smaller, potentially by up to or more than an order
        of magnitude:

        >>> Regex(r"(?-u:\\w){1000}")
        Regex('(?-u:\\w){1000}')
    """
    def __copy__(self, /) -> Regex: ...
    def __deepcopy__(self, memo: Any, /) -> Regex: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __getnewargs_ex__(self, /) -> tuple[tuple[str], dict[str, object]]: ...
    def __hash__(self, /) -> int: ...
    def __new__(
        cls,
        pattern: str,
        /,
        *,
        unicode: bool = True,
        case_insensitive: bool = False,
        multi_line: bool = False,
        dot_matches_new_line: bool = False,
        crlf: bool = False,
        line_terminator: str = "\n",
        swap_greed: bool = False,
        ignore_whitespace: bool = False,
        octal: bool = False,
        size_limit: SupportsIndex | None = None,
        dfa_size_limit: SupportsIndex | None = None,
        nest_limit: SupportsIndex | None = None,
    ) -> Regex: ...
    def __repr__(self, /) -> str: ...
    @property
    def capture_names(self, /) -> list[str | None]:
        """
        The capture names in this regex.

        The list contains values for all capture groups, even ones that are unnamed (``None``). The order
        of the groups corresponds to the order of the group's corresponding opening parenthesis.

        The first element of the list always corresponds to the overall match, and this group is always
        unnamed. Therefore, the list always contains at least one group.

        Examples:
            This shows basic usage with a mix of named and unnamed capture groups (the ``(?:.)`` group is
            non-capturing and so doesn't appear here):

            >>> Regex(r"(?<a>.(?<b>.))(.)(?:.)(?<c>.)").capture_names
            [None, 'a', 'b', None, 'c']

            The list always contains at least one element, even for regexes with no capture groups and even
            for regexes that can never match:

            >>> Regex(r"").capture_names
            [None]
            >>> Regex(r"[a&&b]").capture_names
            [None]
        """
    def captures(self, haystack: str, /, start: SupportsIndex = 0) -> Captures | None:
        """
        This routine searches for the first match of this regex in the haystack given, and if found,
        returns not only the overall match but also the matches of each capture group in the regex. If no
        match is found, then ``None`` is returned.

        Capture group ``0`` always corresponds to an implicit unnamed group that includes the entire match.
        If a match is found, this group is always present. Subsequent groups may be named and are
        numbered, starting at 1, by the order in which the opening parenthesis appears in the pattern. For
        example, in the pattern ``(?<a>.(?<b>.))(?<c>.)``, ``a``, ``b`` and ``c`` correspond to capture
        group indices ``1``, ``2`` and ``3``, respectively.

        You should only use ``captures`` if you need access to the capture group matches. Otherwise,
        :meth:`find` is generally faster for discovering just the overall match.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Say you have some haystack with movie names and their release years, like "'Citizen Kane'
            (1941)". It'd be nice if we could search for substrings looking like that, while also
            extracting the movie name and its release year separately. The example below shows how to do
            that.

            >>> regex = Regex(r"'([^']+)'\\s+\\((\\d{4})\\)")
            >>> caps = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).")
            >>> caps.get(0).text, caps.get(1).text, caps.get(2).text
            ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
            >>> # You can also access the groups by index using the subscript notation.
            >>> caps[0].text, caps[1].text, caps[2].text
            ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')

            Note that the full match is at capture group ``0``. Each subsequent capture group is indexed by
            the order of its opening ``(``.

            We can make this example a bit clearer by using *named* capture groups:

            >>> regex = Regex(r"'(?<title>[^']+)'\\s+\\((?<year>\\d{4})\\)")
            >>> caps = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).")
            >>> caps.get(0).text, caps.name("title").text, caps.name("year").text
            ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')
            >>> # You can also access the groups by name using the subscript notation.
            >>> caps[0].text, caps["title"].text, caps["year"].text
            ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')

            Here we name the capture groups, which we can access with the ``name`` method or the subscript
            notation with a ``str``. Note that the named capture groups are still accessible with ``get``
            or the subscript notation with an ``int``.

            The capture group at index ``0`` is always unnamed, so it must always be accessed with
            ``get(0)`` or ``[0]``.

            Finally, one other way to get the matched substrings is with the :meth:`Captures.extract` API:

            >>> regex = Regex(r"'([^']+)'\\s+\\((\\d{4})\\)")
            >>> full, (title, year) = regex.captures("Not my favorite movie: 'Citizen Kane' (1941).").extract()
            >>> full, title, year
            ("'Citizen Kane' (1941)", 'Citizen Kane', '1941')

            This example shows the significance of ``start`` by demonstrating how it can be used to permit
            look-around assertions in a regex to take the surrounding context into account.

            >>> regex = Regex(r"\bchew\b")
            >>> # We get a match here, but it's probably not intended.
            >>> regex.captures("eschew"[2:])[0].text
            'chew'
            >>> # No match because the assertions take the context into account.
            >>> regex.captures("eschew", 2) is None
            True
        """
    def captures_iter(self, haystack: str, /) -> CaptureMatches:
        r"""
        Returns an iterator that yields successive non-overlapping matches in the given haystack. The
        iterator yields values of type :class:`Captures`.

        This is the same as :meth:`find_iter`, but instead of only providing access to the overall match,
        each value yield includes access to the matches of all capture groups in the regex. Reporting this
        extra match data is potentially costly, so callers should only use ``captures_iter`` over
        ``find_iter`` when they actually need access to the capture group matches.

        Time complexity:
            Note that since ``captures_iter`` runs potentially many searches on the haystack and since each search
            has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
            iteration is ``O(m * n^2)``.

        Examples:
            We can use this to find all movie titles and their release years in some haystack, where the
            movie is formatted like "'Title' (xxxx)":

            >>> regex = Regex(r"'([^']+)'\s+\(([0-9]{4})\)")
            >>> haystack = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931)."
            >>> movies = []
            >>> for _, (title, year) in (caps.extract() for caps in regex.captures_iter(haystack)):
            ...     movies.append((title, int(year)))
            >>> movies
            [('Citizen Kane', 1941), ('The Wizard of Oz', 1939), ('M', 1931)]

            Or with named groups:

            >>> regex = Regex(r"'(?<title>[^']+)'\s+\((?<year>[0-9]{4})\)")
            >>> it = regex.captures_iter(haystack)
            >>> caps = next(it)
            >>> caps["title"].text, caps["year"].text
            ('Citizen Kane', '1941')
            >>> caps = next(it)
            >>> caps["title"].text, caps["year"].text
            ('The Wizard of Oz', '1939')
            >>> caps = next(it)
            >>> caps["title"].text, caps["year"].text
            ('M', '1931')
        """
    @property
    def captures_len(self, /) -> int:
        """
        The number of capture groups in this regex.

        This includes all named and unnamed groups, including the implicit unnamed group that is always
        present and corresponds to the entire match.

        Since the implicit unnamed group is always included in this length, the length is guaranteed to be
        greater than zero.

        Example:
            >>> Regex(r"foo").captures_len
            1
            >>> Regex(r"(foo)").captures_len
            2
            >>> Regex(r"(?<a>.(?<b>.))(.)(?:.)(?<c>.)").captures_len
            5
            >>> Regex(r"[a&&b]").captures_len
            1
        """
    def find(self, haystack: str, /, start: SupportsIndex = 0) -> Match | None:
        """
        This routine searches for the first match of this regex in the haystack given, and if found,
        returns a :class:`Match`. The :class:`Match` provides access to both the offsets of the match and
        the actual substring that matched.

        Note that this should only be used if you want to find the entire match. If instead you just want
        to test the existence of a match, it's potentially faster to use ``regex.is_match(haystack)``
        instead of ``regex.find(haystack) is not None``.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Find the first word with exactly 13 Unicode word characters:

            >>> m = Regex(r"\b\\w{13}\b").find("I categorically deny having triskaidekaphobia.")
            >>> m.span, m.text
            ((2, 15), 'categorically')

            This example shows the significance of ``start`` by demonstrating how it can be used to permit
            look-around assertions in a regex to take the surrounding context into account.

            >>> regex = Regex(r"\bchew\b")
            >>> # We get a match here, but it's probably not intended.
            >>> regex.find("eschew"[2:]).span
            (0, 4)
            >>> # No match because the assertions take the context into account.
            >>> regex.find("eschew", 2) is None
            True
        """
    def find_iter(self, haystack: str, /) -> Matches:
        """
        Returns an iterator that yields successive non-overlapping matches in the given haystack. The
        iterator yields values of type :class:`Match`.

        Time complexity:
            Note that since ``find_iter`` runs potentially many searches on the haystack and since each search
            has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
            iteration is ``O(m * n^2)``.

        Example:
            Find every word with exactly 13 Unicode word characters:

            >>> haystack = "Retroactively relinquishing remunerations is reprehensible."
            >>> [m.text for m in Regex(r"\b\\w{13}\b").find_iter(haystack)]
            ['Retroactively', 'relinquishing', 'remunerations', 'reprehensible']
        """
    def is_match(self, haystack: str, /, start: SupportsIndex = 0) -> bool:
        """
        Returns ``True`` if and only if there is a match for the regex anywhere in the haystack given.

        It is recommended to use this method if all you need to do is test whether a match exists, since
        the underlying matching engine may be able to do less work.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Test if some haystack contains at least one word with exactly 13 Unicode word characters:

            >>> Regex(r"\b\\w{13}\b").is_match("I categorically deny having triskaidekaphobia.")
            True

            This example shows the significance of ``start`` by demonstrating how it can be used to permit
            look-around assertions in a regex to take the surrounding context into account.

            >>> regex = Regex(r"\bchew\b")
            >>> # We get a match here, but it's probably not intended.
            >>> regex.is_match("eschew"[2:])
            True
            >>> # No match because the assertions take the context into account.
            >>> regex.is_match("eschew", 2)
            False
        """
    @property
    def pattern(self, /) -> str:
        """
        The original string of this regex.

        Example:
            >>> Regex(r"foo\\w+bar").pattern
            'foo\\w+bar'
        """
    def replace(
        self, haystack: str, replacement: str | collections.abc.Callable[[Captures], str], /, *, literal: bool = False
    ) -> str:
        r"""
        Replaces the leftmost-first match in the given haystack with the replacement provided. The
        replacement can be a regular string (where ``$N`` and ``$name`` are expanded to match capture
        groups) or a function that takes a :class:`Captures` and returns the replaced string.

        If no match is found, then the haystack is returned unchanged.

        Args:
            haystack: The haystack to search.
            replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
                match capture groups), or a function that takes a :class:`Captures` and returns the
                replaced string.
            literal: Force literal string replacement, without expanding ``$name`` to their
                corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
                example) and faster (since capture groups don't need to be found). Only applies to string
                replacements.

        Raises:
            TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
                a function.

        Replacement string syntax:
            All instances of ``$ref`` in the replacement string are replaced with the substring
            corresponding to the capture group identified by ``ref``.

            ``ref`` may be an integer corresponding to the index of the capture group (counted by order of
            opening parenthesis where ``0`` is the entire match) or it can be a name (consisting of letters,
            digits or underscores) corresponding to a named capture group.

            If ``ref`` isn't a valid capture group (whether the name doesn't exist or isn't a valid index),
            then it is replaced with the empty string.

            The longest possible name is used. For example, ``$1a`` looks up the capture group named ``1a``
            and not the capture group at index ``1``. To exert more precise control over the name, use
            braces, e.g., ``${1}a``.

            To write a literal ``$`` use ``$$``.

        Examples:
            In typical usage, the replacement can just be a normal string:

            >>> Regex(r"[^01]+").replace("1078910", "")
            '1010'

            But a function will work too. It is given the :class:`Captures` corresponding to a match,
            which allows one to access capturing group matches easily:

            >>> regex = Regex(r"([^,\s]+),\s+(\S+)")
            >>> regex.replace("Springsteen, Bruce", lambda caps: f"{caps[2].text} {caps[1].text}")
            'Bruce Springsteen'

            But this is a bit cumbersome to use all the time. Instead, a simple syntax is supported (as
            described above) that expands ``$name`` into the corresponding capture group. Here's the last
            example, but using this expansion technique with named capture groups:

            >>> Regex(r"(?<last>[^,\s]+),\s+(?<first>\S+)").replace("Springsteen, Bruce", "$first $last")
            'Bruce Springsteen'

            Note that using ``$2`` instead of ``$first`` or ``$1`` instead of ``$last`` would produce the
            same result. To write a literal ``$`` use ``$$``.

            Sometimes the replacement string requires use of curly braces to delineate a capture group
            replacement when it is adjacent to some other literal text. For example, if we wanted to join
            two words together with an underscore:

            >>> Regex(r"(?<first>\w+)\s+(?<second>\w+)").replace("deep fried", "${first}_$second")
            'deep_fried'

            Without the curly braces, the capture group name ``first_`` would be used, and since it doesn't
            exist, it would be replaced with the empty string.

            Finally, sometimes you just want to replace a literal string with no regard for capturing group
            expansion. This can be done with ``literal=True``:

            >>> Regex(r"(?<last>[^,\s]+),\s+(\S+)").replace("Springsteen, Bruce", "$2 $last", literal=True)
            '$2 $last'

            Using ``literal=True`` may also be faster, since the replacement string won't need to be parsed
            for the ``$`` syntax.
        """
    def replace_all(
        self, haystack: str, replacement: str | collections.abc.Callable[[Captures], str], /, *, literal: bool = False
    ) -> str:
        """
        Replaces all non-overlapping matches in the haystack with the replacement provided. This is the
        same as calling :meth:`replacen` with ``limit`` set to ``0``.

        If no match is found, then the haystack is returned unchanged.

        The documentation for :meth:`replace` goes into more detail about what kinds of replacement strings
        are supported.

        Args:
            haystack: The haystack to search.
            replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
                match capture groups), or a function that takes a :class:`Captures` and returns the
                replaced string.
            literal: Force literal string replacement, without expanding ``$name`` to their
                corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
                example) and faster (since capture groups don't need to be found). Only applies to string
                replacements.

        Raises:
            TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
                a function.

        Time complexity:
            Since iterators over all matches requires running potentially many searches on the haystack,
            and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
            complexity for this routine is ``O(m * n^2)``.

        Fallibility:
            If a replacement function raises an exception, the exception is only raised once the search
            is over, because there's no way to stop the search process if a replacement fails. Instead, if
            you need to stop as soon as a replacement fails, you should consider implementing your own
            replacement routine:

            >>> def replace_all(regex, haystack, replacement):
            ...     new = []
            ...     last_match = 0
            ...     for caps in regex.captures_iter(haystack):
            ...         m = caps.get_match()
            ...         new.append(haystack[last_match : m.start])
            ...         new.append(replacement(caps))
            ...         last_match = m.end
            ...     new.append(haystack[last_match:])
            ...     return "".join(new)

            Let's replace each word with the number of characters in that word. But if we see a word that
            is "too long," we'll give up.

            >>> def replacement(caps):
            ...     if len(caps[0]) >= 5:
            ...         raise ValueError("word too long")
            ...     return str(len(caps[0]))
            >>> replace_all(Regex(r"\\w+"), "hi how are you?", replacement)
            '2 3 3 3?'
            >>> replace_all(Regex(r"\\w+"), "hi there", replacement)
            Traceback (most recent call last):
            ValueError: word too long

        Example:
            This example shows how to flip the order of whitespace (excluding line terminators) delimited
            fields, and normalizes the whitespace that delimits the fields:

            >>> haystack = (
            ...     "Greetings  1973\nWild\t1973\nBornToRun\t\t\t\t1975\n"
            ...     "Darkness                    1978\nTheRiver 1980"
            ... )
            >>> regex = Regex(r"(?m)^(\\S+)[\\s--\r\n]+(\\S+)$")
            >>> print(regex.replace_all(haystack, "$2 $1"))
            1973 Greetings
            1973 Wild
            1975 BornToRun
            1978 Darkness
            1980 TheRiver
        """
    def replacen(
        self,
        haystack: str,
        limit: SupportsIndex,
        replacement: str | collections.abc.Callable[[Captures], str],
        /,
        *,
        literal: bool = False,
    ) -> str:
        """
        Replaces at most ``limit`` non-overlapping matches in the haystack with the replacement provided.
        If ``limit`` is ``0``, then all non-overlapping matches are replaced. That is,
        ``regex.replace_all(haystack, replacement)`` is equivalent to
        ``regex.replacen(haystack, 0, replacement)``.

        If no match is found, then the haystack is returned unchanged.

        The documentation for :meth:`replace` goes into more detail about what kinds of replacement strings
        are supported.

        Args:
            haystack: The haystack to search.
            limit: The maximum number of matches to replace.
            replacement: The replacement: either a string (where ``$N`` and ``$name`` are expanded to
                match capture groups), or a function that takes a :class:`Captures` and returns the
                replaced string.
            literal: Force literal string replacement, without expanding ``$name`` to their
                corresponding capture groups. This can be both convenient (to avoid escaping ``$``, for
                example) and faster (since capture groups don't need to be found). Only applies to string
                replacements.

        Raises:
            TypeError: If ``replacement`` is neither a string nor a callable, or if ``literal`` is set with
                a function.

        Time complexity:
            Since iterators over all matches requires running potentially many searches on the haystack,
            and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
            complexity for this routine is ``O(m * n^2)``.

            Although note that the worst case time here has an upper bound given by the ``limit``
            parameter.

        Fallibility:
            See the corresponding section in the docs for :meth:`replace_all` for tips on how to deal with
            a replacement routine that can fail.

        Example:
            This example shows how to flip the order of whitespace (excluding line terminators) delimited
            fields, and normalizes the whitespace that delimits the fields. But we only do it for the first
            two matches.

            >>> haystack = (
            ...     "Greetings  1973\nWild\t1973\nBornToRun\t\t\t\t1975\n"
            ...     "Darkness                    1978\nTheRiver 1980"
            ... )
            >>> regex = Regex(r"(?m)^(\\S+)[\\s--\r\n]+(\\S+)$")
            >>> print(regex.replacen(haystack, 2, "$2 $1"))  # doctest: +NORMALIZE_WHITESPACE
            1973 Greetings
            1973 Wild
            BornToRun 1975
            Darkness 1978
            TheRiver 1980
        """
    def shortest_match(self, haystack: str, /, start: SupportsIndex = 0) -> int | None:
        """
        Returns the end offset of the first match in the haystack given.

        This method may have the same performance characteristics as :meth:`is_match`. Behaviorally, it
        doesn't just report whether it match occurs, but also the end offset for a match. In particular,
        the offset returned *may be shorter* than the proper end of the leftmost-first match that you would
        find via :meth:`find`.

        Note that it is not guaranteed that this routine finds the shortest or "earliest" possible match.
        Instead, the main idea of this API is that it returns the offset at the point at which the internal
        regex engine has determined that a match has occurred. This may vary depending on which internal
        regex engine is used, and thus, the offset itself may change based on internal heuristics.

        If a match is found, the offset returned is relative to the beginning of the haystack, not the
        beginning of the search.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Typically, ``a+`` would match the entire first sequence of ``a`` in some haystack, but
            ``shortest_match`` *may* give up as soon as it sees the first ``a``.

            >>> Regex(r"a+").shortest_match("aaaaa")
            1

            This example shows the significance of ``start`` by demonstrating how it can be used to permit
            look-around assertions in a regex to take the surrounding context into account.

            >>> regex = Regex(r"\bchew\b")
            >>> # We get a match here, but it's probably not intended.
            >>> regex.shortest_match("eschew"[2:])
            4
            >>> # No match because the assertions take the context into account.
            >>> regex.shortest_match("eschew", 2) is None
            True
        """
    def split(self, haystack: str, /) -> Split:
        """
        Returns an iterator of substrings of the haystack given, delimited by a match of the regex. Namely,
        each element of the iterator corresponds to a part of the haystack that *isn't* matched by the
        regular expression.

        Time complexity:
            Since iterators over all matches requires running potentially many searches on the haystack,
            and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
            complexity for this routine is ``O(m * n^2)``.

        Examples:
            To split a string delimited by arbitrary amounts of spaces or tabs:

            >>> list(Regex(r"[ \t]+").split("a b \t  c\td    e"))
            ['a', 'b', 'c', 'd', 'e']

            Basic usage:

            >>> list(Regex(r" ").split("Mary had a little lamb"))
            ['Mary', 'had', 'a', 'little', 'lamb']
            >>> list(Regex(r"X").split(""))
            ['']
            >>> list(Regex(r"X").split("lionXXtigerXleopard"))
            ['lion', '', 'tiger', 'leopard']
            >>> list(Regex(r"::").split("lion::tiger::leopard"))
            ['lion', 'tiger', 'leopard']

            If a haystack contains multiple contiguous matches, you will end up with empty spans yielded by
            the iterator:

            >>> list(Regex(r"X").split("XXXXaXXbXc"))
            ['', '', '', '', 'a', '', 'b', 'c']
            >>> list(Regex(r"/").split("(///)"))
            ['(', '', '', ')']

            Separators at the start or end of a haystack are neighbored by empty substring.

            >>> list(Regex(r"0").split("010"))
            ['', '1', '']

            When the empty string is used as a regex, it splits at every valid UTF-8 boundary by default
            (which includes the beginning and end of the haystack):

            >>> list(Regex(r"").split("rust"))
            ['', 'r', 'u', 's', 't', '']
            >>> # Splitting by an empty string is UTF-8 aware by default!
            >>> list(Regex(r"").split("☃"))
            ['', '☃', '']

            Contiguous separators (commonly shows up with whitespace), can lead to possibly surprising
            behavior. For example, this code is correct:

            >>> list(Regex(r" ").split("    a  b c"))
            ['', '', '', '', 'a', '', 'b', 'c']

            It does *not* give you ``['a', 'b', 'c']``. For that behavior, you'd want to match contiguous
            space characters (note that this does still include a leading empty span because ``' +'``
            matches at the beginning of the haystack):

            >>> list(Regex(r" +").split("    a  b c"))
            ['', 'a', 'b', 'c']
        """
    def splitn(self, haystack: str, limit: SupportsIndex, /) -> SplitN:
        r"""
        Returns an iterator of at most ``limit`` substrings of the haystack given, delimited by a match of
        the regex. (A ``limit`` of ``0`` will return no substrings.) Namely, each element of the iterator
        corresponds to a part of the haystack that *isn't* matched by the regular expression. The remainder
        of the haystack that is not split will be the last element in the iterator.

        Time complexity:
            Since iterators over all matches requires running potentially many searches on the haystack,
            and since each search has worst case ``O(m * n)`` time complexity, the overall worst case time
            complexity for this routine is ``O(m * n^2)``.

            Although note that the worst case time here has an upper bound given by the ``limit``
            parameter.

        Examples:
            Get the first two words in some haystack:

            >>> list(Regex(r"\W+").splitn("Hey! How are you?", 3))
            ['Hey', 'How', 'are you?']

            More cases:

            >>> list(Regex(r" ").splitn("Mary had a little lamb", 3))
            ['Mary', 'had', 'a little lamb']
            >>> list(Regex(r"X").splitn("", 3))
            ['']
            >>> list(Regex(r"X").splitn("lionXXtigerXleopard", 3))
            ['lion', '', 'tigerXleopard']
            >>> list(Regex(r"::").splitn("lion::tiger::leopard", 2))
            ['lion', 'tiger::leopard']
            >>> list(Regex(r"X").splitn("abcXdef", 1))
            ['abcXdef']
            >>> list(Regex(r"X").splitn("abcdef", 2))
            ['abcdef']
            >>> list(Regex(r"X").splitn("abcXdef", 0))
            []
        """
    @property
    def static_captures_len(self, /) -> int | None:
        """
        The total number of capturing groups that appear in every possible match.

        If the number of capture groups can vary depending on the match, then this is ``None``. That is, a
        value is only given when the number of matching groups is invariant or "static."

        Note that like :attr:`captures_len`, this **does** include the implicit capturing group
        corresponding to the entire match. Therefore, when a non-``None`` value is given, it is guaranteed
        to be at least ``1``. Stated differently, a value of ``0`` is impossible.

        Example:
            This shows a few cases where a static number of capture groups is available and a few cases
            where it is not.

            >>> patterns = ["a", "(a)", "(a)|(b)", "(a)(b)|(c)(d)", "(a)|b", "a|(b)", "(b)*", "(b)+"]
            >>> [Regex(pattern).static_captures_len for pattern in patterns]
            [1, 2, 2, 3, None, None, None, 2]
        """

@final
class RegexSet:
    r"""
    Match multiple, possibly overlapping, regexes in a single search.

    A regex set corresponds to the union of zero or more regular expressions. That is, a regex set will
    match a haystack when at least one of its constituent regexes matches. A regex set as its
    formulated here provides a touch more power: it will also report *which* regular expressions in
    the set match. Indeed, this is the key difference between regex sets and a single :class:`Regex`
    with many alternates, since only one alternate can match at a time.

    For example, consider regular expressions to match email addresses and domains:
    ``[a-z]+@[a-z]+\.(com|org|net)`` and ``[a-z]+\.(com|org|net)``. If a regex set is constructed
    from those regexes, then searching the haystack ``foo@example.com`` will report both regexes as
    matching. Of course, one could accomplish this by compiling each regex on its own and doing two
    searches over the haystack. The key advantage of using a regex set is that it will report the
    matching regexes using a *single pass through the haystack*. If one has hundreds or thousands of
    regexes to match repeatedly (like a URL router for a complex web application or a user agent
    matcher), then a regex set *can* realize huge performance gains.

    Create a new regex set with the given regular expressions. If any of the patterns are not valid
    regular expressions, then an error is raised.

    Args:
        patterns: An iterable of the regular expressions to compile.
        unicode: See :class:`Regex`.
        case_insensitive: See :class:`Regex`.
        multi_line: See :class:`Regex`.
        dot_matches_new_line: See :class:`Regex`.
        crlf: See :class:`Regex`.
        line_terminator: See :class:`Regex`.
        swap_greed: See :class:`Regex`.
        ignore_whitespace: See :class:`Regex`.
        octal: See :class:`Regex`.
        size_limit: See :class:`Regex`.
        dfa_size_limit: See :class:`Regex`.
        nest_limit: See :class:`Regex`.

    Raises:
        PatternSyntaxError: If any of the patterns is invalid.
        CompiledTooBigError: If the patterns are valid, but would produce a regex set that is bigger
            than the configured size limit via ``size_limit``.

    Limitations:
        Regex sets are limited to answering the following two questions:

        1. Does any regex in the set match?
        2. If so, which regexes in the set match?

        As with the main :class:`Regex` type, it is cheaper to ask (1) instead of (2) since the matching
        engines can stop after the first match is found.

        You cannot directly extract :class:`Match` or :class:`Captures` objects from a regex set. If
        you need these operations, the recommended approach is to compile each pattern in the set
        independently and scan the exact same haystack a second time with those independently compiled
        patterns:

        >>> # Compile a set matching any of our patterns.
        >>> regex_set = RegexSet(["foo", "bar"])
        >>> # Compile each pattern independently.
        >>> regexes = [Regex(pattern) for pattern in regex_set.patterns]
        >>> # Both patterns will match different ranges of this string.
        >>> haystack = "barfoo"
        >>> [regexes[index].find(haystack).text for index in regex_set.matches(haystack)]
        ['foo', 'bar']

        Matches arrive in the order the constituent patterns were declared, not the order they appear
        in the haystack.

    Performance:
        A :class:`RegexSet` has the same performance characteristics as :class:`Regex`. Namely, search
        takes ``O(m * n)`` time, where ``m`` is proportional to the size of the regex set and ``n`` is
        proportional to the length of the haystack.

    Examples:
        This shows how the above two regexes (for matching email addresses and domains) might work:

        >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
        >>> # Ask whether any regexes in the set match.
        >>> regex_set.is_match("foo@example.com")
        True
        >>> # Identify which regexes in the set match.
        >>> list(regex_set.matches("foo@example.com"))
        [0, 1]
        >>> # Try again, but with a haystack that only matches one of the regexes.
        >>> list(regex_set.matches("example.com"))
        [1]
        >>> # Try again, but with a haystack that doesn't match any regex in the set.
        >>> list(regex_set.matches("example"))
        []

        Note that it would be possible to adapt the above example to using :class:`Regex` with an
        expression like:

        .. code-block:: regex

            (?P<email>[a-z]+@(?P<email_domain>[a-z]+[.](com|org|net)))|(?P<domain>[a-z]+[.](com|org|net))

        After a match, one could then inspect the capture groups to figure out which alternates
        matched. The problem is that it is hard to make this approach scale when there are many regexes
        since the overlap between each alternate isn't always obvious to reason about.

        Create a new regex set from an iterable of strings:

        >>> RegexSet([r"\w+", r"\d+"]).is_match("foo")
        True
    """
    def __copy__(self, /) -> RegexSet: ...
    def __deepcopy__(self, memo: Any, /) -> RegexSet: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __getnewargs_ex__(self, /) -> tuple[tuple[list[str]], dict[str, object]]: ...
    def __hash__(self, /) -> int: ...
    def __len__(self, /) -> int:
        """
        Returns the total number of regexes in this set.

        Example:
            >>> len(RegexSet.empty())
            0
            >>> len(RegexSet([r"[0-9]"]))
            1
            >>> len(RegexSet([r"[0-9]", r"[a-z]"]))
            2
        """
    def __new__(
        cls,
        patterns: collections.abc.Iterable[str],
        /,
        *,
        unicode: bool = True,
        case_insensitive: bool = False,
        multi_line: bool = False,
        dot_matches_new_line: bool = False,
        crlf: bool = False,
        line_terminator: str = "\n",
        swap_greed: bool = False,
        ignore_whitespace: bool = False,
        octal: bool = False,
        size_limit: SupportsIndex | None = None,
        dfa_size_limit: SupportsIndex | None = None,
        nest_limit: SupportsIndex | None = None,
    ) -> RegexSet: ...
    def __repr__(self, /) -> str: ...
    @classmethod
    def empty(cls, /) -> RegexSet:
        """
        Create a new empty regex set.

        An empty regex never matches anything.

        Example:
            >>> regex_set = RegexSet.empty()
            >>> len(regex_set)
            0
            >>> # An empty set matches nothing.
            >>> regex_set.is_match("")
            False
        """
    def is_match(self, haystack: str, /, start: SupportsIndex = 0) -> bool:
        """
        Returns ``True`` if and only if one of the regexes in this set matches the haystack given.

        This method should be preferred if you only need to test whether any of the regexes in the set
        should match, but don't care about *which* regexes matched. This is because the underlying matching
        engine will quit immediately after seeing the first match instead of continuing to find all
        matches.

        Note that as with searches using :class:`Regex`, the expression is unanchored by default. That is,
        if the regex does not start with ``^`` or ``\\A``, or end with ``$`` or ``\\z``, then it is permitted
        to match anywhere in the haystack.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Tests whether a set matches somewhere in a haystack:

            >>> regex_set = RegexSet([r"\\w+", r"\\d+"])
            >>> regex_set.is_match("foo")
            True
            >>> regex_set.is_match("☃")
            False

            This example shows the significance of ``start``. Namely, consider a haystack ``foobar`` and a
            desire to execute a search starting at offset ``3``. You could search a substring explicitly,
            but then the look-around assertions won't work correctly. Instead, you can use ``start`` to
            specify the start position of a search.

            >>> regex_set = RegexSet([r"\bbar\b", r"(?m)^bar$"])
            >>> # We get a match here, but it's probably not intended.
            >>> regex_set.is_match("foobar"[3:])
            True
            >>> # No match because the assertions take the context into account.
            >>> regex_set.is_match("foobar", 3)
            False
        """
    def matches(self, haystack: str, /, start: SupportsIndex = 0) -> SetMatches:
        """
        Returns the set of regexes that match in the given haystack.

        The set returned contains the index of each regex that matches in the given haystack. The index is
        in correspondence with the order of regular expressions given to the :class:`RegexSet`
        constructor.

        The set can also be used to iterate over the matched indices. The order of iteration is always
        ascending with respect to the matching indices.

        Note that as with searches using :class:`Regex`, the expression is unanchored by default. That is,
        if the regex does not start with ``^`` or ``\\A``, or end with ``$`` or ``\\z``, then it is permitted
        to match anywhere in the haystack.

        Args:
            haystack: The haystack to search.
            start: The offset at which the search starts. The significance of the starting point is
                that it takes the surrounding context into consideration. For example, the ``\\A``
                anchor can only match when ``start == 0``.

        Raises:
            ValueError: When ``start`` is greater than the length of the haystack.

        Examples:
            Tests which regular expressions match the given haystack:

            >>> regex_set = RegexSet([r"\\w+", r"\\d+", r"\\pL+", r"foo", r"bar", r"barfoo", r"foobar"])
            >>> list(regex_set.matches("foobar"))
            [0, 2, 3, 4, 6]

            You can also test whether a particular regex matched:

            >>> matches = regex_set.matches("foobar")
            >>> matches.matched(5)
            False
            >>> matches.matched(6)
            True

            This example shows the significance of ``start``:

            >>> regex_set = RegexSet([r"\bbar\b", r"(?m)^bar$"])
            >>> # We get matches here, but it's probably not intended.
            >>> list(regex_set.matches("foobar"[3:]))
            [0, 1]
            >>> # No matches because the assertions take the context into account.
            >>> list(regex_set.matches("foobar", 3))
            []
        """
    @property
    def patterns(self, /) -> list[str]:
        """
        The regex patterns that this regex set was constructed from.

        This can be used to determine the pattern for a match. The list has exactly as many patterns given
        to this regex set, and the order of the list is the same as the order of the patterns provided to
        the set.

        Example:
            >>> regex_set = RegexSet([r"\\w+", r"\\d+", r"\\pL+", r"foo", r"bar", r"barfoo", r"foobar"])
            >>> [regex_set.patterns[index] for index in regex_set.matches("foobar")]
            ['\\w+', '\\pL+', 'foo', 'bar', 'foobar']
        """

@final
class SetMatches:
    """
    A set of matches returned by a regex set.

    Iterating over it yields the indices of the regexes that matched, in ascending order, and its
    length is the number of regexes that matched.

    Values of this type are constructed by :meth:`RegexSet.matches`.
    """
    def __contains__(self, index: SupportsIndex, /) -> bool: ...
    def __iter__(self, /) -> SetMatchesIter: ...
    def __len__(self, /) -> int: ...
    def __repr__(self, /) -> str: ...
    def matched(self, /, index: SupportsIndex) -> bool:
        r"""
        Whether the regex at the given index matched.

        The index for a regex is determined by its insertion order upon the initial construction of a
        :class:`RegexSet`, starting at ``0``.

        Raises:
            IndexError: If ``index`` is greater than or equal to the number of regexes in the original set
                that produced these matches.

        Example:
            >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
            >>> matches = regex_set.matches("example.com")
            >>> matches.matched(0)
            False
            >>> matches.matched(1)
            True
        """
    def matched_all(self, /) -> bool:
        r"""
        Whether all patterns in this set matched.

        Example:
            >>> regex_set = RegexSet([r"^foo", r"[a-z]+\.com"])
            >>> regex_set.matches("foo.example.com").matched_all()
            True
        """
    def matched_any(self, /) -> bool:
        r"""
        Whether this set contains any matches.

        Example:
            >>> regex_set = RegexSet([r"[a-z]+@[a-z]+\.(com|org|net)", r"[a-z]+\.(com|org|net)"])
            >>> regex_set.matches("foo@example.com").matched_any()
            True
        """

@final
class SetMatchesIter:
    r"""
    An iterator over the set of matches from a regex set.

    This will always produces matches in ascending order of index, where the index corresponds to the
    index of the regex that matched with respect to its position when initially building the set.

    This iterator is created by iterating over a :class:`SetMatches`, which is automatically done in
    ``for`` loops.

    Example:
        >>> regex_set = RegexSet([r"[0-9]", r"[a-z]", r"[A-Z]", r"\p{Greek}"])
        >>> [index for index in regex_set.matches("βa1")]
        [0, 1, 3]
    """
    def __iter__(self, /) -> SetMatchesIter: ...
    def __next__(self, /) -> int: ...

@final
class Split:
    """
    An iterator over all substrings delimited by a regex match.

    This iterator is created by :meth:`Regex.split`.

    Time complexity:
        Note that since an iterator runs potentially many searches on the haystack and since each search
        has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
        iteration is ``O(m * n^2)``.
    """
    def __iter__(self, /) -> Split: ...
    def __next__(self, /) -> str: ...

@final
class SplitN:
    """
    An iterator over at most ``limit`` substrings delimited by a regex match.

    The last substring yielded by this iterator will be whatever remains after ``limit - 1`` splits.

    This iterator is created by :meth:`Regex.splitn`.

    Time complexity:
        Note that since an iterator runs potentially many searches on the haystack and since each search
        has worst case ``O(m * n)`` time complexity, the overall worst case time complexity for
        iteration is ``O(m * n^2)``.

        Although note that the worst case time here has an upper bound given by the ``limit``
        parameter to :meth:`Regex.splitn`.
    """
    def __iter__(self, /) -> SplitN: ...
    def __next__(self, /) -> str: ...

@final
class SubCaptureMatches:
    """
    An iterator over all group matches in a :class:`Captures` value.

    This iterator yields values of type ``Match | None``. The order of elements yielded corresponds to
    the order of the opening parenthesis for the group in the regex pattern. ``None`` is yielded for
    groups that did not participate in the match.

    The first element always corresponds to the implicit group for the overall match. Since this
    iterator is created by a :class:`Captures` value, and a :class:`Captures` value is only created
    when a match occurs, it follows that the first element yielded by this iterator is guaranteed to
    be non-``None``.

    This iterator is created by iterating over a :class:`Captures`.
    """
    def __iter__(self, /) -> SubCaptureMatches: ...
    def __next__(self, /) -> Match | None: ...

def escape(pattern: str) -> str:
    """
    Escapes all regular expression meta characters in ``pattern``.

    The string returned may be safely used as a literal in a regular expression.

    Example:
        >>> escape("1.5-2.0?")
        '1\\.5\\-2\\.0\\?'
    """
