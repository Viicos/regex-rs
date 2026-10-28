# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

import importlib.metadata
from typing import ClassVar

from pygments.lexer import RegexLexer, bygroups
from pygments.token import Keyword, Name, Operator, Punctuation, String, Whitespace
from sphinx.application import Sphinx

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = "regex-rs"
copyright = "2026-%Y, Victorien"
author = "Victorien"
release = importlib.metadata.version("regexrs-py")

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.intersphinx",
    "sphinx.ext.napoleon",
    "sphinx_issues",
    "sphinx_paramlinks",
    "rtfc.sphinx",
]

rtfc_config_directory = "../.."

autodoc_member_order = "bysource"

# Sections used by the docstrings, which follow the documentation of the `regex` crate.
napoleon_custom_sections = [
    "Fallibility",
    "Format",
    "Limitations",
    "Numbering",
    "Performance",
    "Replacement string syntax",
    "Time complexity",
]

autodoc_typehints = "none"  # Signatures are rendered from the compiled module, which has no annotations.

exclude_patterns = []

linkcheck_ignore = [
    # crates.io only serves its pages to browsers, and answers 404 to the link checker:
    r"https://crates\.io/",
]

intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
}

issues_github_path = "Viicos/regex-rs"

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_baseurl = "https://regex-rs.readthedocs.io/en/latest/"
html_theme = "furo"
html_theme_options = {
    "source_repository": "https://github.com/Viicos/regex-rs/",
    "source_branch": "main",
    "source_directory": "docs/source/",
    "footer_icons": [
        {
            "name": "GitHub",
            "url": "https://github.com/Viicos/regex-rs/",
            "html": """
                <svg stroke="currentColor" fill="currentColor" stroke-width="0" viewBox="0 0 16 16">
                    <path fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"></path>
                </svg>
            """,
            "class": "",
        },
    ],
}


# -- Regex lexer ---------------------------------------------------------------


class RegexPatternLexer(RegexLexer):
    """Highlight patterns written in the syntax of the Rust ``regex`` crate.

    See https://docs.rs/regex/latest/regex/#syntax.
    """

    name = "Regex"
    aliases = ("regex",)

    # Escapes valid both inside and outside character classes.
    _escapes: ClassVar[list[tuple[str, object]]] = [
        (r"\\[pP](\{[^}]*\}|[A-Za-z])", Name.Builtin),  # Unicode classes: `\pL`, `\p{Greek}`.
        (r"\\[dDsSwW]", Name.Builtin),  # Perl classes.
        (r"\\(x\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8})", String.Escape),
        (r"\\[0-7]{1,3}", String.Escape),  # Octal, if enabled.
        (r"\\[aftnrv]", String.Escape),
        (r"\\.", String.Escape),  # Escaped literal (e.g. `\.`), or assertion outside classes.
    ]

    tokens: ClassVar[dict[str, list[object]]] = {
        "root": [
            (r"\\b\{[^}]*\}|\\[bBAz<>]", Keyword),  # Assertions: `\b{start}`, `\b`, `\A`, `\z`, `\<`.
            *_escapes,
            (r"(\(\?P?<)([A-Za-z_][\w.\[\]]*)(>)", bygroups(Keyword, Name.Label, Keyword)),  # Named groups.
            (r"\(\?[a-zA-Z-]*:?", Keyword),  # Non-capturing groups and flags: `(?:`, `(?i)`, `(?-u:`.
            (r"[()]", Keyword),
            (r"\[\^?\]?", Punctuation, "class"),
            (r"(\{\d+(,\d*)?\}|[*+?])\??", Operator),  # Repetitions, possibly lazy.
            (r"[|]", Operator),
            (r"[\^$.]", Keyword.Pseudo),
            (r"\s+", Whitespace),
            (r"[^\\()\[\]{}*+?|^$.\s]+", String.Regex),
            (r".", String.Regex),
        ],
        "class": [
            (r"\[:\^?[a-z]+:\]", Name.Builtin),  # ASCII classes: `[:alpha:]`.
            *_escapes,
            (r"\[\^?\]?", Punctuation, "#push"),  # Nested classes.
            (r"\]", Punctuation, "#pop"),
            (r"&&|--|~~|-", Operator),  # Set operations and ranges.
            (r"[^\\\[\]&~-]+", String.Regex),
            (r".", String.Regex),
        ],
    }


def setup(app: Sphinx) -> None:
    app.add_lexer("regex", RegexPatternLexer)
