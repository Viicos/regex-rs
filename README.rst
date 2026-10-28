========
regex-rs
========

|Pythons| |PyPI| |Docs| |Ruff|

.. |Pythons| image:: https://img.shields.io/pypi/pyversions/regexrs-py.svg
  :alt: Supported Python versions
  :target: https://pypi.org/project/regexrs-py/

.. |PyPI| image:: https://img.shields.io/pypi/v/regexrs-py.svg
  :alt: PyPI - Version
  :target: https://pypi.org/project/regexrs-py/

.. |Docs| image:: https://img.shields.io/readthedocs/regex-rs.svg
  :alt: Documentation status
  :target: https://regex-rs.readthedocs.io

.. |Ruff| image:: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json
  :alt: Ruff
  :target: https://github.com/astral-sh/ruff

Python bindings to the Rust `regex`_ crate.

.. _`regex`: https://docs.rs/regex

Installation
------------

From PyPI:

.. code:: bash

    pip install regexrs-py

Usage
-----

.. code:: python

    >>> from regex_rs import Regex
    >>> r = Regex(r"(?<year>\d{4})-(?<month>\d{2})")
    >>> r.is_match("released in 2026-10")
    True
    >>> r.find("released in 2026-10")
    <regex_rs.Match span=(12, 19) text='2026-10'>
    >>> r.captures("released in 2026-10")["year"].text
    '2026'
    >>> r.replace_all("2026-10, 2027-01", "$month/$year")
    '10/2026, 01/2027'

The current version of the `regex`_ crate used is `1.13.1 <https://crates.io/crates/regex/1.13.1>`__.
