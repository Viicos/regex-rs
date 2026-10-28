use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// The options of `RegexBuilder` and `RegexSetBuilder`, shared by `Regex` and `RegexSet`.
///
/// The limits are optional: the `regex` crate doesn't document their default values, so they are
/// only set when given, and left to the crate otherwise.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Options {
    pub unicode: bool,
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub crlf: bool,
    pub line_terminator: u8,
    pub swap_greed: bool,
    pub ignore_whitespace: bool,
    pub octal: bool,
    pub size_limit: Option<usize>,
    pub dfa_size_limit: Option<usize>,
    pub nest_limit: Option<u32>,
}

impl Default for Options {
    /// The defaults of the `regex` crate, documented by each method of
    /// <https://docs.rs/regex/1.13.1/regex/struct.RegexBuilder.html>.
    fn default() -> Self {
        Self {
            unicode: true,
            case_insensitive: false,
            multi_line: false,
            dot_matches_new_line: false,
            crlf: false,
            line_terminator: b'\n',
            swap_greed: false,
            ignore_whitespace: false,
            octal: false,
            size_limit: None,
            dfa_size_limit: None,
            nest_limit: None,
        }
    }
}

/// Applies `Options` to a `RegexBuilder` or a `RegexSetBuilder`, which share the same methods.
macro_rules! configure {
    ($builder:expr, $options:expr) => {{
        let options: &$crate::options::Options = $options;
        let builder = $builder
            .unicode(options.unicode)
            .case_insensitive(options.case_insensitive)
            .multi_line(options.multi_line)
            .dot_matches_new_line(options.dot_matches_new_line)
            .crlf(options.crlf)
            .line_terminator(options.line_terminator)
            .swap_greed(options.swap_greed)
            .ignore_whitespace(options.ignore_whitespace)
            .octal(options.octal);
        if let Some(limit) = options.size_limit {
            builder.size_limit(limit);
        }
        if let Some(limit) = options.dfa_size_limit {
            builder.dfa_size_limit(limit);
        }
        if let Some(limit) = options.nest_limit {
            builder.nest_limit(limit);
        }
        builder
    }};
}
pub(crate) use configure;

impl Options {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
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
        let line_terminator = match line_terminator.as_bytes() {
            &[byte] if byte.is_ascii() => byte,
            _ => {
                return Err(PyValueError::new_err(format!(
                    "'line_terminator' must be a single ASCII character, got {line_terminator:?}"
                )));
            }
        };
        Ok(Self {
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
        })
    }

    /// The options as keyword arguments, either all of them or only the ones not set to their default.
    pub(crate) fn to_kwargs<'py>(
        &self,
        py: Python<'py>,
        non_default_only: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let default = Self::default();
        let kwargs = PyDict::new(py);
        macro_rules! set {
            ($($field:ident),*) => {$(
                if !non_default_only || self.$field != default.$field {
                    kwargs.set_item(stringify!($field), self.$field)?;
                }
            )*};
        }
        set!(
            unicode,
            case_insensitive,
            multi_line,
            dot_matches_new_line,
            crlf
        );
        if !non_default_only || self.line_terminator != default.line_terminator {
            kwargs.set_item(
                "line_terminator",
                char::from(self.line_terminator).to_string(),
            )?;
        }
        set!(
            swap_greed,
            ignore_whitespace,
            octal,
            size_limit,
            dfa_size_limit,
            nest_limit
        );
        Ok(kwargs)
    }

    /// The options not set to their default, formatted as keyword arguments for a `repr()`.
    pub(crate) fn repr_kwargs(&self, py: Python<'_>) -> PyResult<String> {
        let mut out = String::new();
        for (key, value) in self.to_kwargs(py, true)?.iter() {
            out.push_str(&format!(", {key}={}", value.repr()?));
        }
        Ok(out)
    }
}
