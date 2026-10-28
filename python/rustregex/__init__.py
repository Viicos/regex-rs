"""Python bindings to the Rust `regex <https://docs.rs/regex>`_ crate."""

from ._errors import CompiledTooBigError, PatternSyntaxError, RegexError
from ._rustregex import (
    REGEX_VERSION,
    CaptureMatches,
    Captures,
    Match,
    Matches,
    Regex,
    RegexSet,
    SetMatches,
    SetMatchesIter,
    Split,
    SplitN,
    SubCaptureMatches,
    escape,
)

__all__ = (
    "REGEX_VERSION",
    "CaptureMatches",
    "Captures",
    "CompiledTooBigError",
    "Match",
    "Matches",
    "PatternSyntaxError",
    "Regex",
    "RegexError",
    "RegexSet",
    "SetMatches",
    "SetMatchesIter",
    "Split",
    "SplitN",
    "SubCaptureMatches",
    "escape",
)
