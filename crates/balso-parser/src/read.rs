use crate::{Action, Error, Follower, Scanner};
use balso_core::*;

pub fn read(string: &str, follower: &mut impl Follower) -> Result<(), Error> {
    let mut scanner = Scanner::new(string);

    sequence(None, &mut scanner, follower)?;

    if scanner.is_done() {
        Ok(())
    } else {
        Err(Error::Character(scanner.cursor()))
    }
}

fn sequence(input: Option<&BondKind>, scanner: &mut Scanner, reporter: &mut impl Follower) -> Result<bool, Error> {
    let atom_kind = match atom(scanner)? {
        Some(kind) => kind,
        None => return Ok(false),
    };

    match &input {
        Some(bond_kind) => reporter.extend(bond_kind, &atom_kind),
        None => reporter.root(&atom_kind),
    }

    loop {
        if union(scanner, reporter)? || branch(scanner, reporter)? || gap(scanner, reporter)? {
            continue;
        }

        break Ok(true);
    }
}

fn union(scanner: &mut Scanner, reporter: &mut impl Follower) -> Result<bool, Error> {
    match bond(scanner) {
        Some(bond_kind) => {
            if bridge_or_sequence(&bond_kind, scanner, reporter)? {
                Ok(true)
            } else {
                Err(missing_character(scanner))
            }
        }
        None => bridge_or_sequence(&BondKind::Elided, scanner, reporter),
    }
}

fn bridge_or_sequence(
    bond_kind: &BondKind,
    scanner: &mut Scanner,
    reporter: &mut impl Follower,
) -> Result<bool, Error> {
    if let Some(bridge) = bridge(scanner)? {
        reporter.bridge(bond_kind, &bridge);

        Ok(true)
    } else {
        sequence(Some(bond_kind), scanner, reporter)
    }
}

fn branch(scanner: &mut Scanner, reporter: &mut impl Follower) -> Result<bool, Error> {
    if !scanner.take(&'(') {
        return Ok(false);
    }

    reporter.push();

    if scanner.take(&'.') {
        if !sequence(None, scanner, reporter)? {
            return Err(missing_character(scanner));
        }
    } else {
        let bond_kind = match bond(scanner) {
            Some(bond_kind) => bond_kind,
            None => BondKind::Elided,
        };

        if !sequence(Some(&bond_kind), scanner, reporter)? {
            return Err(missing_character(scanner));
        }
    }

    if !scanner.take(&')') {
        return Err(missing_character(scanner));
    }

    reporter.pop();

    if union(scanner, reporter)? || branch(scanner, reporter)? {
        Ok(true)
    } else {
        Err(missing_character(scanner))
    }
}

fn gap(scanner: &mut Scanner, reporter: &mut impl Follower) -> Result<bool, Error> {
    if !scanner.take(&'.') {
        return Ok(false);
    }

    if sequence(None, scanner, reporter)? {
        Ok(true)
    } else {
        Err(missing_character(scanner))
    }
}

fn atom(scanner: &mut Scanner) -> Result<Option<AtomKind>, Error> {
    if scanner.take(&'*') {
        Ok(Some(AtomKind::Star))
    } else if let Some(shortcut) = shortcut(scanner)? {
        Ok(Some(AtomKind::Shortcut(shortcut)))
    } else if let Some(selection) = selection(scanner) {
        Ok(Some(AtomKind::Selection(selection)))
    } else if let Some(bracket) = bracket(scanner)? {
        Ok(Some(AtomKind::Bracket(bracket)))
    } else {
        Ok(None)
    }
}

fn bond(scanner: &mut Scanner) -> Option<BondKind> {
    scanner.transform(|target| match target {
        '-' => Some(BondKind::Single),
        '=' => Some(BondKind::Double),
        '#' => Some(BondKind::Triple),
        '/' => Some(BondKind::Up),
        '\\' => Some(BondKind::Down),
        _ => None,
    })
}

fn nonzero(scanner: &mut Scanner) -> Option<u8> {
    scanner.transform(|character| match character {
        '1' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        '4' => Some(4),
        '5' => Some(5),
        '6' => Some(6),
        '7' => Some(7),
        '8' => Some(8),
        '9' => Some(9),
        _ => None,
    })
}

fn digit(scanner: &mut Scanner) -> Option<u8> {
    if scanner.take(&'0') {
        Some(0)
    } else {
        nonzero(scanner)
    }
}

fn missing_character(scanner: &mut Scanner) -> Error {
    if scanner.is_done() {
        Error::EndOfLine
    } else {
        Error::Character(scanner.cursor())
    }
}

fn selection(scanner: &mut Scanner) -> Option<Selection> {
    scanner.transform(|character| match character {
        'b' => Some(Selection::B),
        'c' => Some(Selection::C),
        'n' => Some(Selection::N),
        'o' => Some(Selection::O),
        'p' => Some(Selection::P),
        's' => Some(Selection::S),
        _ => None,
    })
}

fn shortcut(scanner: &mut Scanner) -> Result<Option<Shortcut>, Error> {
    Ok(scanner.scan(|symbol| match symbol {
        "B" => Some(Action::Request(Shortcut::B)),
        "Br" => Some(Action::Return(Shortcut::Br)),
        "C" => Some(Action::Request(Shortcut::C)),
        "Cl" => Some(Action::Return(Shortcut::Cl)),
        "N" => Some(Action::Return(Shortcut::N)),
        "O" => Some(Action::Return(Shortcut::O)),
        "F" => Some(Action::Return(Shortcut::F)),
        "I" => Some(Action::Return(Shortcut::I)),
        "P" => Some(Action::Return(Shortcut::P)),
        "S" => Some(Action::Return(Shortcut::S)),
        _ => None,
    })?)
}

fn element(scanner: &mut Scanner) -> Result<Option<Element>, Error> {
    Ok(scanner.scan(|symbol| {
        let bytes = symbol.as_bytes();
        match bytes.len() {
            1 => match bytes[0] {
                b'A' | b'E' | b'G' | b'L' | b'M' | b'R' | b'X' | b'Z' => Some(Action::Require),
                b'B' => Some(Action::Request(Element::B)),
                b'C' => Some(Action::Request(Element::C)),
                b'D' => Some(Action::Request(Element::D)),
                b'F' => Some(Action::Request(Element::F)),
                b'H' => Some(Action::Request(Element::H)),
                b'I' => Some(Action::Request(Element::I)),
                b'K' => Some(Action::Request(Element::K)),
                b'N' => Some(Action::Request(Element::N)),
                b'O' => Some(Action::Request(Element::O)),
                b'P' => Some(Action::Request(Element::P)),
                b'S' => Some(Action::Request(Element::S)),
                b'T' => Some(Action::Request(Element::T)),
                b'U' => Some(Action::Return(Element::U)),
                b'V' => Some(Action::Return(Element::V)),
                b'W' => Some(Action::Return(Element::W)),
                b'Y' => Some(Action::Request(Element::Y)),
                _ => None,
            },
            2 => {
                let first = bytes[0];
                let second = bytes[1];

                match first {
                    b'A' => match second {
                        b'c' => Some(Action::Return(Element::Ac)),
                        b'g' => Some(Action::Return(Element::Ag)),
                        b'l' => Some(Action::Return(Element::Al)),
                        b'm' => Some(Action::Return(Element::Am)),
                        b'r' => Some(Action::Return(Element::Ar)),
                        b's' => Some(Action::Return(Element::As)),
                        b't' => Some(Action::Return(Element::At)),
                        b'u' => Some(Action::Return(Element::Au)),
                        _ => None,
                    },
                    b'B' => match second {
                        b'a' => Some(Action::Return(Element::Ba)),
                        b'e' => Some(Action::Return(Element::Be)),
                        b'i' => Some(Action::Return(Element::Bi)),
                        b'k' => Some(Action::Return(Element::Bk)),
                        b'r' => Some(Action::Return(Element::Br)),
                        _ => None,
                    },
                    b'C' => match second {
                        b'a' => Some(Action::Return(Element::Ca)),
                        b'd' => Some(Action::Return(Element::Cd)),
                        b'e' => Some(Action::Return(Element::Ce)),
                        b'f' => Some(Action::Return(Element::Cf)),
                        b'l' => Some(Action::Return(Element::Cl)),
                        b'm' => Some(Action::Return(Element::Cm)),
                        b'o' => Some(Action::Return(Element::Co)),
                        b'r' => Some(Action::Return(Element::Cr)),
                        b's' => Some(Action::Return(Element::Cs)),
                        b'u' => Some(Action::Return(Element::Cu)),
                        _ => None,
                    },
                    b'E' => match second {
                        b'r' => Some(Action::Return(Element::Er)),
                        b's' => Some(Action::Return(Element::Es)),
                        b'u' => Some(Action::Return(Element::Eu)),
                        _ => None,
                    },
                    b'F' => match second {
                        b'e' => Some(Action::Return(Element::Fe)),
                        b'm' => Some(Action::Return(Element::Fm)),
                        b'r' => Some(Action::Return(Element::Fr)),
                        _ => None,
                    },
                    b'G' => match second {
                        b'a' => Some(Action::Return(Element::Ga)),
                        b'd' => Some(Action::Return(Element::Gd)),
                        b'e' => Some(Action::Return(Element::Ge)),
                        _ => None,
                    },
                    b'H' => match second {
                        b'e' => Some(Action::Return(Element::He)),
                        b'f' => Some(Action::Return(Element::Hf)),
                        b'g' => Some(Action::Return(Element::Hg)),
                        b'o' => Some(Action::Return(Element::Ho)),
                        _ => None,
                    },
                    b'L' => match second {
                        b'a' => Some(Action::Return(Element::La)),
                        b'i' => Some(Action::Return(Element::Li)),
                        b'r' => Some(Action::Return(Element::Lr)),
                        b'u' => Some(Action::Return(Element::Lu)),
                        _ => None,
                    },
                    b'M' => match second {
                        b'd' => Some(Action::Return(Element::Md)),
                        b'g' => Some(Action::Return(Element::Mg)),
                        b'n' => Some(Action::Return(Element::Mn)),
                        b'o' => Some(Action::Return(Element::Mo)),
                        _ => None,
                    },
                    b'N' => match second {
                        b'a' => Some(Action::Return(Element::Na)),
                        b'b' => Some(Action::Return(Element::Nb)),
                        b'd' => Some(Action::Return(Element::Nd)),
                        b'e' => Some(Action::Return(Element::Ne)),
                        b'i' => Some(Action::Return(Element::Ni)),
                        b'o' => Some(Action::Return(Element::No)),
                        b'p' => Some(Action::Return(Element::Np)),
                        _ => None,
                    },
                    b'P' => match second {
                        b'a' => Some(Action::Return(Element::Pa)),
                        b'b' => Some(Action::Return(Element::Pb)),
                        b'd' => Some(Action::Return(Element::Pd)),
                        b'm' => Some(Action::Return(Element::Pm)),
                        b'o' => Some(Action::Return(Element::Po)),
                        b'r' => Some(Action::Return(Element::Pr)),
                        b't' => Some(Action::Return(Element::Pt)),
                        b'u' => Some(Action::Return(Element::Pu)),
                        _ => None,
                    },
                    b'R' => match second {
                        b'a' => Some(Action::Return(Element::Ra)),
                        b'b' => Some(Action::Return(Element::Rb)),
                        b'e' => Some(Action::Return(Element::Re)),
                        b'f' => Some(Action::Return(Element::Rf)),
                        b'h' => Some(Action::Return(Element::Rh)),
                        b'n' => Some(Action::Return(Element::Rn)),
                        b'u' => Some(Action::Return(Element::Ru)),
                        _ => None,
                    },
                    b'S' => match second {
                        b'b' => Some(Action::Return(Element::Sb)),
                        b'c' => Some(Action::Return(Element::Sc)),
                        b'e' => Some(Action::Return(Element::Se)),
                        b'i' => Some(Action::Return(Element::Si)),
                        b'm' => Some(Action::Return(Element::Sm)),
                        b'n' => Some(Action::Return(Element::Sn)),
                        b'r' => Some(Action::Return(Element::Sr)),
                        _ => None,
                    },
                    b'T' => match second {
                        b'a' => Some(Action::Return(Element::Ta)),
                        b'b' => Some(Action::Return(Element::Tb)),
                        b'c' => Some(Action::Return(Element::Tc)),
                        b'e' => Some(Action::Return(Element::Te)),
                        b'h' => Some(Action::Return(Element::Th)),
                        b'i' => Some(Action::Return(Element::Ti)),
                        b'l' => Some(Action::Return(Element::Tl)),
                        b'm' => Some(Action::Return(Element::Tm)),
                        _ => None,
                    },
                    // Merge Dy, In, Ir, Kr, Os, Xe, Yb, Zn, Zr
                    _ => match symbol {
                        "Dy" => Some(Action::Return(Element::Dy)),
                        "In" => Some(Action::Return(Element::In)),
                        "Ir" => Some(Action::Return(Element::Ir)),
                        "Kr" => Some(Action::Return(Element::Kr)),
                        "Os" => Some(Action::Return(Element::Os)),
                        "Xe" => Some(Action::Return(Element::Xe)),
                        "Yb" => Some(Action::Return(Element::Yb)),
                        "Zn" => Some(Action::Return(Element::Zn)),
                        "Zr" => Some(Action::Return(Element::Zr)),
                        _ => None,
                    },
                }
            }
            _ => None,
        }
    })?)
}

fn bridge(scanner: &mut Scanner) -> Result<Option<Bridge>, Error> {
    if scanner.take(&'%') {
        if let Some(first) = nonzero(scanner) {
            if let Some(second) = digit(scanner) {
                Ok(Some(Bridge::new(first * 10 + second).expect("bridge index")))
            } else {
                Err(missing_character(scanner))
            }
        } else {
            Err(missing_character(scanner))
        }
    } else if let Some(digit) = nonzero(scanner) {
        Ok(Some(Bridge::new(digit).expect("bridge index")))
    } else {
        Ok(None)
    }
}

fn isotope(scanner: &mut Scanner) -> Option<Isotope> {
    let mut sum = match nonzero(scanner) {
        Some(digit) => digit as u16,
        None => return None,
    };

    for _ in 0..2 {
        sum = match digit(scanner) {
            Some(digit) => sum * 10 + digit as u16,
            None => return Some(Isotope::new(sum).expect("isotope")),
        };
    }

    Some(Isotope::new(sum).expect("isotope"))
}

fn symbol(scanner: &mut Scanner) -> Result<Option<Symbol>, Error> {
    if let Some(element) = element(scanner)? {
        Ok(Some(Symbol::Element(element)))
    } else if let Some(selection) = selection(scanner) {
        Ok(Some(Symbol::Selection(selection)))
    } else if star(scanner) {
        Ok(Some(Symbol::Star))
    } else {
        Ok(None)
    }
}

fn star(scanner: &mut Scanner) -> bool {
    scanner.take(&'*')
}

fn atom_parity(scanner: &mut Scanner) -> Option<AtomParity> {
    if scanner.take(&'@') {
        if scanner.take(&'@') {
            Some(AtomParity::Clockwise)
        } else {
            Some(AtomParity::Counterclockwise)
        }
    } else {
        None
    }
}

fn virtual_hydrogen(scanner: &mut Scanner) -> Option<VirtualHydrogen> {
    if scanner.take(&'H') {
        match nonzero(scanner) {
            Some(digit) => Some(VirtualHydrogen::new(digit).expect("digit")),
            _ => Some(VirtualHydrogen::default()),
        }
    } else {
        None
    }
}

fn charge(scanner: &mut Scanner) -> Option<Charge> {
    if scanner.take(&'+') {
        match nonzero(scanner) {
            Some(digit) => Some(Charge::new(digit as i8).expect("charge")),
            None => Some(Charge::Plus),
        }
    } else if scanner.take(&'-') {
        match nonzero(scanner) {
            Some(digit) => Some(Charge::new(digit as i8 * -1).expect("charge")),
            None => Some(Charge::Minus),
        }
    } else {
        None
    }
}

fn bracket(scanner: &mut Scanner) -> Result<Option<Bracket>, Error> {
    if !scanner.take(&'[') {
        return Ok(None);
    }

    let result = Ok(Some(Bracket {
        isotope: isotope(scanner),
        symbol: match symbol(scanner)? {
            Some(symbol) => symbol,
            None => return Err(missing_character(scanner)),
        },
        parity: atom_parity(scanner),
        hydrogens: virtual_hydrogen(scanner),
        charge: charge(scanner),
    }));

    if scanner.take(&']') {
        result
    } else {
        Err(missing_character(scanner))
    }
}

#[cfg(test)]
mod read {
    use crate::follow::Writer;

    use super::*;

    #[test]
    fn blank() {
        let mut writer = Writer::new();

        read("", &mut writer).unwrap();

        assert_eq!(writer.write(), "")
    }

    #[test]
    fn leading_paren() {
        let mut writer = Writer::new();

        assert_eq!(read("(", &mut writer), Err(Error::Character(0)))
    }

    #[test]
    fn invalid_tail() {
        let mut writer = Writer::new();

        assert_eq!(read("*?", &mut writer), Err(Error::Character(1)))
    }

    #[test]
    fn trailing_bond() {
        let mut writer = Writer::new();

        assert_eq!(read("*-", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn trailing_dot() {
        let mut writer = Writer::new();

        assert_eq!(read("*.", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn open_paran_eol() {
        let mut writer = Writer::new();

        assert_eq!(read("*(", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn missing_close_paren() {
        let mut writer = Writer::new();

        assert_eq!(read("*(*", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn bond_to_invalid() {
        let mut writer = Writer::new();

        assert_eq!(read("*-X", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn split_to_invalid() {
        let mut writer = Writer::new();

        assert_eq!(read("*.X", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn bond_dot() {
        let mut writer = Writer::new();

        assert_eq!(read("*-.", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn branch_invalid() {
        let mut writer = Writer::new();

        assert_eq!(read("*(X", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn branch_rnum() {
        let mut writer = Writer::new();

        assert_eq!(read("*(1)*", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn branch_bond_rnum() {
        let mut writer = Writer::new();

        assert_eq!(read("*(-1", &mut writer), Err(Error::Character(3)))
    }

    #[test]
    fn dot_rnum() {
        let mut writer = Writer::new();

        assert_eq!(read("*.1", &mut writer), Err(Error::Character(2)))
    }

    #[test]
    fn branch_split_eol() {
        let mut writer = Writer::new();

        assert_eq!(read("*(.", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn branch_split_invalid() {
        let mut writer = Writer::new();

        assert_eq!(read("*(.x", &mut writer), Err(Error::Character(3)))
    }

    #[test]
    fn trailing_branch() {
        let mut writer = Writer::new();

        assert_eq!(read("*(*)", &mut writer), Err(Error::EndOfLine))
    }

    #[test]
    fn gap_after_branch() {
        let mut writer = Writer::new();

        assert_eq!(read("*(*).*", &mut writer), Err(Error::Character(4)))
    }

    #[test]
    fn p1() {
        let mut writer = Writer::new();

        read("*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*")
    }

    #[test]
    fn shortcut_c() {
        let mut writer = Writer::new();

        read("C", &mut writer).unwrap();

        assert_eq!(writer.write(), "C")
    }

    #[test]
    fn shortcut_c_selected() {
        let mut writer = Writer::new();

        read("cc", &mut writer).unwrap();

        assert_eq!(writer.write(), "cc")
    }

    #[test]
    fn shortcut_cl() {
        let mut writer = Writer::new();

        read("Cl", &mut writer).unwrap();

        assert_eq!(writer.write(), "Cl")
    }

    #[test]
    fn bracket() {
        let mut writer = Writer::new();

        read("[CH4]", &mut writer).unwrap();

        assert_eq!(writer.write(), "[CH4]")
    }

    #[test]
    fn elided_cut() {
        let mut writer = Writer::new();

        read("*1", &mut writer).unwrap();

        assert_eq!(writer.write(), "*1")
    }

    #[test]
    fn single_cut() {
        let mut writer = Writer::new();

        read("*-1", &mut writer).unwrap();

        assert_eq!(writer.write(), "*-1")
    }

    #[test]
    fn p1_p1() {
        let mut writer = Writer::new();

        read("*.*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*.*")
    }

    #[test]
    fn p1_p2_branched_inner() {
        let mut writer = Writer::new();

        read("*(.*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(.*)*")
    }

    #[test]
    fn p2_elided() {
        let mut writer = Writer::new();

        read("**", &mut writer).unwrap();

        assert_eq!(writer.write(), "**")
    }

    #[test]
    fn p2_single() {
        let mut writer = Writer::new();

        read("*-*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*-*")
    }

    #[test]
    fn p3_elided() {
        let mut writer = Writer::new();

        read("***", &mut writer).unwrap();

        assert_eq!(writer.write(), "***")
    }

    #[test]
    fn p3_branched_elided() {
        let mut writer = Writer::new();

        read("*(F)Cl", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(F)Cl")
    }

    #[test]
    fn p4_branched_elided() {
        let mut writer = Writer::new();

        read("*(**)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(**)*")
    }

    #[test]
    fn p4_branched_outside() {
        let mut writer = Writer::new();

        read("*(-*)=**", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(-*)=**")
    }

    #[test]
    fn s3_internal() {
        let mut writer = Writer::new();

        read("*(*)(*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(*)(*)*")
    }

    #[test]
    fn double_nested() {
        let mut writer = Writer::new();

        read("*(*(*-*)*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(*(*-*)*)*")
    }

    #[test]
    fn triple_nested() {
        let mut writer = Writer::new();

        read("*(-*(=*)*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(-*(=*)*)*")
    }

    #[test]
    fn s4_internal() {
        let mut writer = Writer::new();

        read("*(-*)(=*)(#*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "*(-*)(=*)(#*)*")
    }

    #[test]
    fn s4_external() {
        let mut writer = Writer::new();

        read("**(-*)(=*)*", &mut writer).unwrap();

        assert_eq!(writer.write(), "**(-*)(=*)*")
    }

    #[test]
    fn branch_ordering() {
        let mut writer = Writer::new();

        read("C(F)Cl", &mut writer).unwrap();

        assert_eq!(writer.write(), "C(F)Cl")
    }
}
