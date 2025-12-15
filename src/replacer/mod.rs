use crate::error::{Error, Result};
use regex::bytes::{Regex, RegexBuilder};
use std::borrow::Cow;

mod validate;

pub struct Replacer {
    regex: Regex,
    replacement: Vec<u8>,
    max_replacements: usize,
    // TODO: track validation mode (strict, warn, none)
}

impl Replacer {
    pub fn new(
        pattern: &str,
        replacement: &str,
        fixed_strings: bool,
        ignore_case: bool,
        smart_case: bool,
        _case_sensitive: bool,
        word_regexp: bool,
        multiline: bool,
        single_line: bool,
        dot_matches_newline: bool,
        no_unicode: bool,
        _crlf: bool,
        max_replacements: usize,
    ) -> Result<Self> {
        // 1. Validate replacement pattern for capture group references
        validate::validate_replacement(replacement)?;

        // 2. Build regex
        let pattern = if fixed_strings {
            regex::escape(pattern)
        } else {
            pattern.to_string()
        };

        let pattern = if word_regexp {
            format!(r"\b{}\b", pattern)
        } else {
            pattern
        };

        let mut builder = RegexBuilder::new(&pattern);
        builder.unicode(!no_unicode);

        // Case handling: priority: ignore_case, smart_case, case_sensitive (default)
        if ignore_case {
            builder.case_insensitive(true);
        } else if smart_case {
            // Smart case: case insensitive if pattern is all lowercase
            let is_lowercase = pattern.chars().all(|c| !c.is_uppercase());
            builder.case_insensitive(is_lowercase);
        } else {
            builder.case_insensitive(false);
        }

        builder.multi_line(multiline && !single_line);
        builder.dot_matches_new_line(dot_matches_newline);
        // TODO: handle crlf mode (requires special handling)

        let regex = builder.build().map_err(Error::Regex)?;

        // 3. Process replacement string (unescape $$, etc.)
        let replacement_bytes = replacement.as_bytes().to_vec(); // TODO: unescape

        Ok(Self {
            regex,
            replacement: replacement_bytes,
            max_replacements,
        })
    }

    pub fn replace<'a>(&self, text: &'a [u8]) -> Cow<'a, [u8]> {
        if self.max_replacements == 0 {
            self.regex.replace_all(text, &*self.replacement)
        } else {
            self.regex.replacen(text, self.max_replacements, &*self.replacement)
        }
    }

    /// Count the number of matches in the given text.
    pub fn count_matches(&self, text: &[u8]) -> usize {
        self.regex.find_iter(text).count()
    }

    /// Replace matches in text and return the replaced text along with the number of replacements performed.
    pub fn replace_with_count<'a>(&self, text: &'a [u8]) -> (Cow<'a, [u8]>, usize) {
        let matches = self.count_matches(text);
        let replaced = if self.max_replacements == 0 {
            self.regex.replace_all(text, &*self.replacement)
        } else {
            self.regex.replacen(text, self.max_replacements, &*self.replacement)
        };
        let actual_replacements = if self.max_replacements == 0 {
            matches
        } else {
            std::cmp::min(matches, self.max_replacements)
        };
        (replaced, actual_replacements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_replacement() {
        let replacer = Replacer::new(
            "foo",
            "bar",
            false, // fixed_strings
            false, // ignore_case
            false, // smart_case
            true,  // case_sensitive
            false, // word_regexp
            false, // multiline
            false, // single_line
            false, // dot_matches_newline
            false, // no_unicode
            false, // crlf
            0,     // max_replacements
        ).unwrap();
        let input = b"foo baz foo";
        let output = replacer.replace(input);
        assert_eq!(&output[..], b"bar baz bar");
    }

    #[test]
    fn test_capture_group() {
        let replacer = Replacer::new(
            r"(\d+)",
            "number-$1",
            false, false, false, true, false, false, false, false, false, false, 0
        ).unwrap();
        let input = b"abc 123 def";
        let output = replacer.replace(input);
        assert_eq!(&output[..], b"abc number-123 def");
    }

    #[test]
    fn test_max_replacements() {
        let replacer = Replacer::new(
            "x",
            "y",
            false, false, false, true, false, false, false, false, false, false, 2
        ).unwrap();
        let input = b"x x x x";
        let output = replacer.replace(input);
        assert_eq!(&output[..], b"y y x x");
    }
}