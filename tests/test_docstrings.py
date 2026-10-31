"""Run the examples of the docstrings, which are defined in the compiled module."""

import doctest
import inspect
from pathlib import Path

import pytest

import regex_rs


def _examples() -> list[doctest.DocTest]:
    # `doctest.DocTestFinder` skips the properties of extension types and objects defined in
    # submodules, so docstrings are collected from the public API instead.
    parser = doctest.DocTestParser()
    tests: list[doctest.DocTest] = []
    for name in regex_rs.__all__:
        obj = getattr(regex_rs, name)
        objects = [(f"regex_rs.{name}", obj)]
        if inspect.isclass(obj):
            # Only methods and properties: other class attributes (such as `__firstlineno__`, an `int` on
            # Python 3.13+) would expose the docstring of their type.
            objects += [
                (f"regex_rs.{name}.{attr}", value)
                for attr, value in vars(obj).items()
                if inspect.isroutine(value) or inspect.isdatadescriptor(value)
            ]
        for qualname, value in objects:
            doc = getattr(value, "__doc__", None)
            if isinstance(doc, str) and ">>>" in doc:
                tests.append(parser.get_doctest(doc, vars(regex_rs).copy(), qualname, None, 0))
    return tests


@pytest.mark.parametrize("test", _examples(), ids=lambda test: test.name)
def test_docstring_examples(test: doctest.DocTest) -> None:
    runner = doctest.DocTestRunner(optionflags=doctest.ELLIPSIS)
    output: list[str] = []
    runner.run(test, out=output.append)
    assert runner.failures == 0, "".join(output)


def test_examples_are_found() -> None:
    # Guards against examples silently not being collected.
    names = {test.name for test in _examples()}
    assert {"regex_rs.Regex", "regex_rs.Regex.find", "regex_rs.Regex.captures_len", "regex_rs.escape"} <= names


def test_readme_examples() -> None:
    readme = Path(__file__).parent.parent / "README.rst"
    results = doctest.testfile(str(readme), module_relative=False, optionflags=doctest.ELLIPSIS)
    assert results.failed == 0
    assert results.attempted > 0
