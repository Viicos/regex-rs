use pyo3::prelude::*;

/// Converts a `regex` compilation error into the matching exception from `regex_rs._errors`.
///
/// The exception classes are defined in Python so that they are fully typed and documented.
pub(crate) fn to_py_err(py: Python<'_>, err: regex::Error) -> PyErr {
    let message = err.to_string();
    let build = || -> PyResult<PyErr> {
        let errors = py.import("regex_rs._errors")?;
        let exc = match err {
            regex::Error::Syntax(_) => errors.getattr("PatternSyntaxError")?.call1((message,))?,
            regex::Error::CompiledTooBig(limit) => errors
                .getattr("CompiledTooBigError")?
                .call1((message, limit))?,
            _ => errors.getattr("RegexError")?.call1((message,))?,
        };
        Ok(PyErr::from_value(exc))
    };
    build().unwrap_or_else(|err| err)
}
