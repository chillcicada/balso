use std::{fmt, fmt::Write};

use super::{Element, Selection};

#[derive(Debug, PartialEq, Clone)]
pub enum Symbol {
    Star,                 // *
    Element(Element),     // e.g. C, N, O
    Selection(Selection), // e.g. c, n, o
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Star => f.write_char('*'),
            Self::Element(element) => element.fmt(f),
            Self::Selection(selection) => selection.fmt(f),
        }
    }
}

impl std::default::Default for Symbol {
    fn default() -> Self {
        Symbol::Star
    }
}
