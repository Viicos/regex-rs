from __future__ import annotations


class RegexError(ValueError):
    """Base class of the errors raised when compiling a regular expression."""


class PatternSyntaxError(RegexError):
    """Raised when a pattern is not a valid regular expression.

    The message points at the location of the error in the pattern.
    """


class CompiledTooBigError(RegexError):
    """Raised when a compiled regular expression would exceed the configured size limit.

    See the ``size_limit`` argument of :class:`~rustregex.Regex`.
    """

    limit: int
    """The size limit that was exceeded, in bytes."""

    def __init__(self, message: str, limit: int) -> None:
        super().__init__(message)
        self.limit = limit

    def __reduce__(self) -> tuple[type[CompiledTooBigError], tuple[str, int]]:
        return (type(self), (self.args[0], self.limit))
