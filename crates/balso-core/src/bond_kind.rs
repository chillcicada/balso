use std::fmt::{self, Write};

#[derive(Debug, PartialEq, Clone)]
pub enum BondKind {
    Elided, //
    Single, // -
    Double, // =
    Triple, // #
    Up,     // /
    Down,   // \
}

impl fmt::Display for BondKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Elided => Ok(()),
            Self::Single => f.write_char('-'),
            Self::Double => f.write_char('='),
            Self::Triple => f.write_char('#'),
            Self::Up => f.write_char('/'),
            Self::Down => f.write_char('\\'),
        }
    }
}

impl BondKind {
    pub fn reverse(&self) -> Self {
        match self {
            BondKind::Elided => Self::Elided,
            BondKind::Single => Self::Single,
            BondKind::Double => Self::Double,
            BondKind::Triple => Self::Triple,
            BondKind::Up => Self::Down,
            BondKind::Down => Self::Up,
        }
    }

    pub fn bond_order(&self) -> u8 {
        match self {
            Self::Elided | Self::Single | Self::Up | Self::Down => 1,
            Self::Double => 2,
            Self::Triple => 3,
        }
    }
}
