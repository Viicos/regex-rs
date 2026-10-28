import copy
import pickle
from concurrent.futures import ThreadPoolExecutor

import pytest

from regex_rs import (
    REGEX_VERSION,
    Captures,
    CompiledTooBigError,
    PatternSyntaxError,
    Regex,
    RegexError,
    RegexSet,
    escape,
)


def test_regex_version() -> None:
    assert REGEX_VERSION.startswith("1.")


class TestCompilation:
    def test_syntax_error(self) -> None:
        with pytest.raises(PatternSyntaxError, match="unclosed group") as exc_info:
            Regex("(")
        assert isinstance(exc_info.value, RegexError)
        assert isinstance(exc_info.value, ValueError)

    @pytest.mark.parametrize("pattern", [r"(?=a)", r"(a)\1"])
    def test_unsupported_features(self, pattern: str) -> None:
        with pytest.raises(PatternSyntaxError):
            Regex(pattern)

    def test_compiled_too_big(self) -> None:
        with pytest.raises(CompiledTooBigError) as exc_info:
            Regex(r"\w{100}", size_limit=1000)
        assert exc_info.value.limit == 1000
        unpickled = pickle.loads(pickle.dumps(exc_info.value))
        assert (unpickled.limit, unpickled.args) == (1000, exc_info.value.args)

    def test_nest_limit(self) -> None:
        Regex("((a))", nest_limit=10)
        with pytest.raises(PatternSyntaxError):
            Regex("((a))", nest_limit=1)

    def test_default_options(self) -> None:
        # Options left to their default are omitted from the repr.
        assert repr(Regex("a")) == "Regex('a')"
        assert repr(RegexSet(["a"])) == "RegexSet(['a'])"

    def test_repr_shows_non_default_options(self) -> None:
        regex = Regex("a", case_insensitive=True, line_terminator="\0", size_limit=100_000)
        assert repr(regex) == "Regex('a', case_insensitive=True, line_terminator='\\x00', size_limit=100000)"

    @pytest.mark.parametrize("line_terminator", ["", "ab", "é"])
    def test_invalid_line_terminator(self, line_terminator: str) -> None:
        with pytest.raises(ValueError, match="line_terminator"):
            Regex("a", line_terminator=line_terminator)


class TestOptions:
    def test_case_insensitive(self) -> None:
        assert Regex("foo", case_insensitive=True).is_match("FOO")
        assert not Regex("foo").is_match("FOO")

    def test_multi_line(self) -> None:
        assert [m.text for m in Regex("^[a-z]+$", multi_line=True).find_iter("foo\nbar")] == ["foo", "bar"]
        assert Regex("^[a-z]+$").find("foo\nbar") is None

    def test_dot_matches_new_line(self) -> None:
        assert Regex("a.b", dot_matches_new_line=True).is_match("a\nb")
        assert not Regex("a.b").is_match("a\nb")

    def test_crlf(self) -> None:
        assert [m.text for m in Regex("^[a-z]+$", multi_line=True, crlf=True).find_iter("foo\r\nbar")] == ["foo", "bar"]

    def test_line_terminator(self) -> None:
        assert not Regex("a.b", line_terminator="\0").is_match("a\0b")
        assert Regex("a.b", line_terminator="\0").is_match("a\nb")

    def test_swap_greed(self) -> None:
        assert Regex("a+", swap_greed=True).find("aaa").text == "a"  # type: ignore[union-attr]

    def test_ignore_whitespace(self) -> None:
        assert Regex("a b # comment", ignore_whitespace=True).is_match("ab")

    def test_octal(self) -> None:
        assert Regex(r"\141", octal=True).is_match("a")
        with pytest.raises(PatternSyntaxError, match="backreferences are not supported"):
            Regex(r"\1")

    def test_unicode(self) -> None:
        assert Regex(r"\w").is_match("é")
        assert not Regex(r"\w", unicode=False).is_match("é")
        with pytest.raises(PatternSyntaxError):
            Regex(".", unicode=False)


class TestSearching:
    def test_is_match(self) -> None:
        regex = Regex(r"\d+")
        assert regex.is_match("abc 123")
        assert not regex.is_match("abc")

    def test_find(self) -> None:
        m = Regex(r"\d+").find("abc 123 456")
        assert m is not None
        assert (m.start, m.end, m.span, m.text, len(m), m.is_empty()) == (4, 7, (4, 7), "123", 3, False)
        assert Regex(r"\d+").find("abc") is None

    def test_offsets_are_code_points(self) -> None:
        haystack = "é☃ 💩 abc"
        m = Regex("abc").find(haystack)
        assert m is not None
        assert m.span == (5, 8)
        assert haystack[m.start : m.end] == "abc"

    def test_start(self) -> None:
        regex = Regex(r"\bchew\b")
        assert regex.find("eschew", 2) is None
        assert regex.find("eschew"[2:]) is not None
        assert not regex.is_match("eschew", 2)
        assert Regex(r"\Aa").find("aa", 1) is None
        m = Regex("a").find("☃a☃a", 2)
        assert m is not None
        assert m.span == (3, 4)

    def test_start_bounds(self) -> None:
        assert Regex("").find("ab", 2) is not None
        with pytest.raises(ValueError, match="start"):
            Regex("a").find("ab", 3)

    def test_shortest_match(self) -> None:
        assert Regex("a+").shortest_match("aaaaa") == 1
        assert Regex("b").shortest_match("☃☃b") == 3
        assert Regex("b").shortest_match("aaa") is None

    def test_empty_match(self) -> None:
        m = Regex("a*").find("b")
        assert m is not None
        assert (m.span, m.text, m.is_empty(), len(m)) == ((0, 0), "", True, 0)

    def test_large_haystack(self) -> None:
        # Large haystacks are searched without holding the GIL.
        haystack = "é" * 10_000 + "needle"
        m = Regex("needle").find(haystack)
        assert m is not None
        assert m.span == (10_000, 10_006)
        assert Regex("needle").replace(haystack, "x") == "é" * 10_000 + "x"

    def test_threads(self) -> None:
        regex = Regex(r"(\w+)@(\w+)\.com")
        haystack = "a@b.com " * 5_000
        with ThreadPoolExecutor(8) as pool:
            counts = list(pool.map(lambda _: sum(1 for _ in regex.captures_iter(haystack)), range(16)))
        assert counts == [5_000] * 16

    def test_match_text_reuses_haystack(self) -> None:
        haystack = "".join(["ab", "c"])  # A non-interned string.
        m = Regex(".*").find(haystack)
        assert m is not None
        assert m.text is haystack

    def test_surrogates(self) -> None:
        with pytest.raises(UnicodeEncodeError):
            Regex("a").is_match("\ud800")


class TestIteration:
    def test_find_iter(self) -> None:
        assert [m.span for m in Regex(r"\d+").find_iter("a1b22c333")] == [(1, 2), (3, 5), (6, 9)]

    @pytest.mark.parametrize(
        ("pattern", "haystack", "expected"),
        [
            ("", "aé☃", [(0, 0), (1, 1), (2, 2), (3, 3)]),
            ("a*", "baaab", [(0, 0), (1, 4), (5, 5)]),
            (r"\b", "ab cd", [(0, 0), (2, 2), (3, 3), (5, 5)]),
            ("a|", "aba", [(0, 1), (2, 3)]),
            ("|a", "aba", [(0, 0), (1, 1), (2, 2), (3, 3)]),
        ],
    )
    def test_empty_matches(self, pattern: str, haystack: str, expected: list[tuple[int, int]]) -> None:
        assert [m.span for m in Regex(pattern).find_iter(haystack)] == expected

    def test_iterators_are_lazy_and_exhaustible(self) -> None:
        matches = Regex("a").find_iter("aa")
        assert iter(matches) is matches
        assert next(matches).span == (0, 1)
        assert next(matches).span == (1, 2)
        assert next(matches, None) is None
        assert next(matches, None) is None

    def test_captures_iter(self) -> None:
        regex = Regex(r"'(?<title>[^']+)'\s+\((?<year>\d{4})\)")
        haystack = "'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931)."
        assert [caps.extract()[1] for caps in regex.captures_iter(haystack)] == [
            ("Citizen Kane", "1941"),
            ("The Wizard of Oz", "1939"),
            ("M", "1931"),
        ]

    @pytest.mark.parametrize(
        ("pattern", "haystack", "expected"),
        [
            (r"[ \t]+", "a b \t  c\td    e", ["a", "b", "c", "d", "e"]),
            ("X", "lionXXtigerXleopard", ["lion", "", "tiger", "leopard"]),
            ("::", "lion::tiger::leopard", ["lion", "tiger", "leopard"]),
            (",", ",a,b,", ["", "a", "b", ""]),
            ("", "rust", ["", "r", "u", "s", "t", ""]),
            ("", "☃", ["", "☃", ""]),
            (" ", "    a  b c", ["", "", "", "", "a", "", "b", "c"]),
            ("x", "", [""]),
        ],
    )
    def test_split(self, pattern: str, haystack: str, expected: list[str]) -> None:
        assert list(Regex(pattern).split(haystack)) == expected

    @pytest.mark.parametrize(
        ("haystack", "limit", "expected"),
        [
            ("Hey! How are you?", 3, ["Hey", "How", "are you?"]),
            ("Hey! How are you?", 1, ["Hey! How are you?"]),
            ("Hey! How are you?", 0, []),
            ("Hey! How are you?", 10, ["Hey", "How", "are", "you", ""]),
            ("", 3, [""]),
        ],
    )
    def test_splitn(self, haystack: str, limit: int, expected: list[str]) -> None:
        assert list(Regex(r"\W+").splitn(haystack, limit)) == expected


class TestCaptures:
    @pytest.fixture
    def caps(self) -> Captures:
        caps = Regex(r"(?<first>\w)(\w)(?:\w)\w(?<last>\w)|(?<other>\d)").captures("toady")
        assert caps is not None
        return caps

    def test_lookups(self, caps: Captures) -> None:
        assert caps[0].text == caps.get_match().text == "toady"  # type: ignore[union-attr]
        assert caps["first"].text == caps.name("first").text == "t"  # type: ignore[union-attr]
        assert caps[2].text == caps.get(2).text == "o"  # type: ignore[union-attr]
        assert caps["other"] is None
        assert caps[4] is None

    def test_unknown_groups(self, caps: Captures) -> None:
        assert caps.get(10) is None
        assert caps.name("unknown") is None
        with pytest.raises(IndexError):
            caps[10]
        with pytest.raises(IndexError):
            caps[-1]
        with pytest.raises(KeyError):
            caps["unknown"]

    def test_len_and_iter(self, caps: Captures) -> None:
        assert len(caps) == 5
        assert [m and m.text for m in caps] == ["toady", "t", "o", "y", None]

    def test_group_offsets(self) -> None:
        # Groups are not ordered by position: the last iteration sets group 1 after group 2.
        caps = Regex("(?:(é)|(☃))+").captures("x☃é")
        assert caps is not None
        assert [m and m.span for m in caps] == [(1, 3), (2, 3), (1, 2)]

    def test_extract(self) -> None:
        caps = Regex(r"(a)(b)|(c)(d)").captures("cd")
        assert caps is not None
        assert caps.extract() == ("cd", ("c", "d"))
        caps = Regex(r"(a)(b)|(c)").captures("c")
        assert caps is not None
        with pytest.raises(ValueError, match="can vary"):
            caps.extract()

    def test_expand(self, caps: Captures) -> None:
        # `$1a` refers to the (unknown) group named `1a`, not to group 1 followed by `a`.
        assert caps.expand("$last$first-${2}x-$1a-$$-$unknown") == "yt-ox--$-"


class TestReplace:
    def test_template(self) -> None:
        regex = Regex(r"(?<last>[^,\s]+),\s+(?<first>\S+)")
        assert regex.replace("Springsteen, Bruce", "$first $last") == "Bruce Springsteen"
        assert regex.replace("Springsteen, Bruce", "$2 $1") == "Bruce Springsteen"
        assert regex.replace("Springsteen, Bruce", "$first $last", literal=True) == "$first $last"

    def test_function(self) -> None:
        assert Regex(r"\d+").replace_all("a1b22", lambda caps: str(int(caps[0].text) * 2)) == "a2b44"  # type: ignore[union-attr]

    def test_function_error(self) -> None:
        calls: list[str] = []

        def replacement(caps: Captures) -> str:
            calls.append(caps.get_match().text)
            raise RuntimeError("boom")

        with pytest.raises(RuntimeError, match="boom"):
            Regex(r"\d").replace_all("1 2 3", replacement)
        assert calls == ["1"]

    def test_function_wrong_return_type(self) -> None:
        with pytest.raises(TypeError):
            Regex("a").replace("a", lambda caps: 1)  # type: ignore[arg-type, return-value]

    def test_invalid_replacement(self) -> None:
        with pytest.raises(TypeError, match="string or a callable"):
            Regex("a").replace("a", 1)  # type: ignore[arg-type]
        with pytest.raises(TypeError, match="literal"):
            Regex("a").replace("a", lambda caps: "b", literal=True)

    def test_limits(self) -> None:
        regex = Regex("a")
        assert regex.replace("aaa", "b") == "baa"
        assert regex.replace_all("aaa", "b") == "bbb"
        assert regex.replacen("aaa", 2, "b") == "bba"
        assert regex.replacen("aaa", 0, "b") == "bbb"

    def test_no_match_returns_haystack(self) -> None:
        haystack = "".join(["x", "yz"])
        assert Regex("a").replace_all(haystack, "b") is haystack

    def test_non_ascii(self) -> None:
        assert Regex("(é)").replace_all("aéb☃é", lambda caps: f"[{caps[1].span}]") == "a[(1, 2)]b☃[(4, 5)]"  # type: ignore[union-attr]


class TestRegexSet:
    def test_matches(self) -> None:
        regex_set = RegexSet([r"\w+", r"\d+", r"\pL+", r"foo", r"bar", r"barfoo", r"foobar"])
        matches = regex_set.matches("foobar")
        assert list(matches) == [0, 2, 3, 4, 6]
        assert len(matches) == 5
        assert 2 in matches
        assert 1 not in matches
        assert 100 not in matches
        assert matches.matched(0)
        assert not matches.matched(1)
        assert matches.matched_any()
        assert not matches.matched_all()
        assert repr(matches) == "<regex_rs.SetMatches [0, 2, 3, 4, 6]>"
        with pytest.raises(IndexError):
            matches.matched(7)

    def test_is_match_and_start(self) -> None:
        regex_set = RegexSet([r"\bbar"])
        assert regex_set.is_match("bar")
        assert not regex_set.is_match("foobar", 3)
        assert not regex_set.matches("foobar", 3).matched_any()

    def test_attributes(self) -> None:
        regex_set = RegexSet(p for p in ["a", "b"])
        assert regex_set.patterns == ["a", "b"]
        assert len(regex_set) == 2

    def test_empty(self) -> None:
        empty = RegexSet.empty()
        assert len(empty) == 0
        assert not empty.is_match("")
        assert empty == RegexSet([])

    def test_string_patterns(self) -> None:
        with pytest.raises(TypeError, match="not a string"):
            RegexSet("abc")

    def test_errors(self) -> None:
        with pytest.raises(PatternSyntaxError):
            RegexSet(["a", "("])

    def test_options(self) -> None:
        assert RegexSet(["foo"], case_insensitive=True).is_match("FOO")


@pytest.mark.parametrize(
    "obj",
    [Regex(r"\w+"), Regex("a", case_insensitive=True, line_terminator="\0"), RegexSet(["a", "b"], multi_line=True)],
)
def test_value_semantics(obj: Regex | RegexSet) -> None:
    assert pickle.loads(pickle.dumps(obj)) == obj
    assert copy.copy(obj) is obj
    assert copy.deepcopy(obj) is obj
    assert hash(pickle.loads(pickle.dumps(obj))) == hash(obj)


def test_equality() -> None:
    assert Regex("a") == Regex("a")
    assert Regex("a") != Regex("b")
    assert Regex("a") != Regex("a", case_insensitive=True)
    assert Regex("a") != "a"
    assert len({Regex("a"), Regex("a"), Regex("b")}) == 2


def test_escape() -> None:
    pattern = escape("1.5-2.0?")
    assert Regex(pattern).find("v1.5-2.0?").text == "1.5-2.0?"  # type: ignore[union-attr]
