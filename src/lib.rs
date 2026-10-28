use pyo3::prelude::*;

mod errors;
mod haystack;
mod iter;
mod matches;
mod options;
mod pyregex;
mod set;

#[pymodule]
mod _rustregex {
    use pyo3::prelude::*;

    #[pymodule_export]
    use crate::iter::{CaptureMatches, Matches, Split, SplitN};
    #[pymodule_export]
    use crate::matches::{Captures, Match, SubCaptureMatches};
    #[pymodule_export]
    use crate::pyregex::Regex;
    #[pymodule_export]
    use crate::set::{RegexSet, SetMatches, SetMatchesIter};

    /// The version of the Rust ``regex`` crate this module was built against.
    #[pymodule_export]
    const REGEX_VERSION: &str = env!("REGEX_VERSION");

    /// Escapes all regular expression meta characters in ``pattern``.
    ///
    /// The string returned may be safely used as a literal in a regular expression.
    ///
    /// Example:
    ///     >>> escape("1.5-2.0?")
    ///     '1\\.5\\-2\\.0\\?'
    #[pyfunction]
    fn escape(pattern: &str) -> String {
        regex::escape(pattern)
    }
}
