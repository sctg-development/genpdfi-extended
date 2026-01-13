// Copyright (c) 2025 Ismael Theiskaa
// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

//! Utilities for text wrapping.

use std::mem;

use crate::style;
use crate::Context;
use crate::Mm;

/// Combines a sequence of styled words into lines with a maximum width.
///
/// If a word does not fit into a line, the wrapper tries to split it using the `split` function.
pub struct Wrapper<'c, 's, I: Iterator<Item = style::StyledStr<'s>>> {
    iter: I,
    context: &'c Context,
    width: Mm,
    x: Mm,
    buf: Vec<style::StyledCow<'s>>,
    has_overflowed: bool,
}

impl<'c, 's, I: Iterator<Item = style::StyledStr<'s>>> Wrapper<'c, 's, I> {
    /// Creates a new wrapper for the given word sequence and with the given maximum width.
    pub fn new(iter: I, context: &'c Context, width: Mm) -> Wrapper<'c, 's, I> {
        Wrapper {
            iter,
            context,
            width,
            x: Mm(0.0),
            buf: Vec::new(),
            has_overflowed: false,
        }
    }

    /// Returns true if this wrapper has overflowed, i. e. if it encountered a word that it could
    /// not split so that it would fit into a line.
    pub fn has_overflowed(&self) -> bool {
        self.has_overflowed
    }
}

impl<'c, 's, I: Iterator<Item = style::StyledStr<'s>>> Iterator for Wrapper<'c, 's, I> {
    // This iterator yields pairs of lines and the length difference between the input words and
    // the line.
    type Item = (Vec<style::StyledCow<'s>>, usize);

    fn next(&mut self) -> Option<(Vec<style::StyledCow<'s>>, usize)> {
        // Append words to self.buf until the maximum line length is reached
        while let Some(s) = self.iter.next() {
            let mut width = s.width(&self.context.font_cache);

            if self.x + width > self.width {
                // The word does not fit into the current line (at least not completely)

                let mut delta = 0;
                // Try to split the word so that the first part fits into the current line
                let s = if let Some((start, end)) = split(self.context, s, self.width - self.x) {
                    // Calculate the number of bytes that we added to the string when splitting it
                    // (for the hyphen, if required).
                    delta = start.s.len() + end.s.len() - s.s.len();
                    self.buf.push(start);
                    width = end.width(&self.context.font_cache);
                    end
                } else {
                    s.into()
                };

                if width > self.width {
                    // The remainder of the word is longer than the current page – we will never be
                    // able to render it completely.
                    // TODO: handle gracefully, emit warning
                    self.has_overflowed = true;
                    return None;
                }

                // Return the current line and add the word that did not fit to the next line
                let v = std::mem::take(&mut self.buf);
                self.buf.push(s);
                self.x = width;
                return Some((v, delta));
            } else {
                // The word fits in the current line, so just append it
                self.buf.push(s.into());
                self.x += width;
            }
        }

        if self.buf.is_empty() {
            None
        } else {
            Some((mem::take(&mut self.buf), 0))
        }
    }
}

#[cfg(not(feature = "hyphenation"))]
fn split<'s>(
    _context: &Context,
    _s: style::StyledStr<'s>,
    _len: Mm,
) -> Option<(style::StyledCow<'s>, style::StyledCow<'s>)> {
    None
}

/// Tries to split the given string into two parts so that the first part is shorter than the given
/// width.
#[cfg(feature = "hyphenation")]
fn split<'s>(
    context: &Context,
    s: style::StyledStr<'s>,
    width: Mm,
) -> Option<(style::StyledCow<'s>, style::StyledCow<'s>)> {
    use hyphenation::{Hyphenator, Iter};

    let hyphenator = if let Some(hyphenator) = &context.hyphenator {
        hyphenator
    } else {
        return None;
    };

    let mark = "-";
    let mark_width = s.style.str_width(&context.font_cache, mark);

    let hyphenated = hyphenator.hyphenate(s.s);
    let segments: Vec<_> = hyphenated.iter().segments().collect();

    // Find the hyphenation with the longest first part so that the first part (and the hyphen) are
    // shorter than or equals to the required width.
    let idx = segments
        .iter()
        .scan(Mm(0.0), |acc, t| {
            *acc += s.style.str_width(&context.font_cache, t);
            Some(*acc)
        })
        .position(|w| w + mark_width > width)
        .unwrap_or_default();
    if idx > 0 {
        let idx = hyphenated.breaks[idx - 1];
        let start = s.s[..idx].to_owned() + mark;
        let end = &s.s[idx..];
        Some((
            style::StyledCow::new(start, s.style, None),
            style::StyledCow::new(end, s.style, None),
        ))
    } else {
        None
    }
}

/// Splits a sequence of styled strings into words.
pub struct Words<I: Iterator<Item = style::StyledString>> {
    iter: I,
    s: Option<style::StyledString>,
    link: Option<String>,
}

impl<I: Iterator<Item = style::StyledString>> Words<I> {
    /// Creates a new words iterator.
    pub fn new<IntoIter: IntoIterator<Item = style::StyledString, IntoIter = I>>(
        iter: IntoIter,
    ) -> Words<I> {
        Words {
            iter: iter.into_iter(),
            s: None,
            link: None,
        }
    }
}

impl<I: Iterator<Item = style::StyledString>> Iterator for Words<I> {
    type Item = style::StyledString;

    fn next(&mut self) -> Option<style::StyledString> {
        if self.s.as_ref().map(|s| s.s.is_empty()).unwrap_or(true) {
            self.s = self.iter.next();
            if let Some(s) = &self.s {
                self.link = s.link.clone();
            }
        }

        if let Some(s) = &mut self.s {
            // Split at the first space or use the complete string
            let n = s.s.find(' ').map(|i| i + 1).unwrap_or_else(|| s.s.len());
            let mut tmp = s.s.split_off(n);
            mem::swap(&mut tmp, &mut s.s);
            Some(style::StyledString::new(tmp, s.style, self.link.clone()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{self, FontCache, FontData, FontFamily};
    use crate::style::{Style, StyledString};
    use crate::Context;
    use std::path::PathBuf;

    fn find_test_font() -> Option<PathBuf> {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let td = manifest.join("testdata");
        if let Ok(entries) = std::fs::read_dir(&td) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf") {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_words_iterator_splits_on_space() {
        let input = vec![StyledString::new(
            "Hello world!".to_owned(),
            Style::new(),
            None,
        )];
        let mut words = Words::new(input.into_iter());
        let w1 = words.next().unwrap();
        assert_eq!(w1.s, "Hello ");
        let w2 = words.next().unwrap();
        assert_eq!(w2.s, "world!");
        assert!(words.next().is_none());
    }

    #[test]
    fn test_wrapper_overflow_sets_flag() {
        // Build a context with a real font to get realistic widths
        // Use bundled test font bytes from the `fonts/` directory to avoid runtime path issues
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fonts/NotoSans-Regular.ttf"
        ))
        .to_vec();
        let fd = FontData::new(data, None).expect("font data");
        let family = FontFamily {
            regular: fd.clone(),
            bold: fd.clone(),
            italic: fd.clone(),
            bold_italic: fd.clone(),
        };
        let cache = FontCache::new(family);
        let context = Context::new(cache);

        let binding = "a".repeat(200);
        let long_word = style::StyledStr::new(&binding, Style::new(), None);
        let mut wrapper = Wrapper::new(std::iter::once(long_word), &context, Mm(0.1));
        assert!(wrapper.next().is_none());
        assert!(wrapper.has_overflowed());
    }
}
