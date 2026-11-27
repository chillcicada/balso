use balso_core::{AtomKind, Charge, Element, Selection, Shortcut};

use super::Vec2;
use crate::Atom;

const SUBSCRIPT_ZERO: u32 = 0x2080;
const SUPERSCRIPT_ZERO: u32 = 0x2070;
const SUPERSCRIPT_ONE: u32 = 0x00B9;
const SUPERSCRIPT_TWO: u32 = 0x00B2;
const SUPERSCRIPT_THREE: u32 = 0x00B3;

/// Text drawn at an atom position.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// Unicode text, with subscripts and superscripts already applied.
    pub text: String,
    /// Font size in SVG user units.
    pub font_size: f64,
}

impl Label {
    fn new(text: String, font_size: f64) -> Self {
        Self { text, font_size }
    }

    /// Approximate rendered width, used for the bond mask.
    pub fn width(&self) -> f64 {
        self.text
            .chars()
            .map(|character| if is_script_character(character) { 0.42 } else { 0.62 })
            .sum::<f64>()
            * self.font_size
    }

    /// Half-height of the mask that keeps bonds out from under the glyphs.
    pub fn mask_height(&self) -> f64 {
        self.font_size * 0.62
    }
}

/// The label for an atom, or `None` when the atom stays implicit (a skeletal
/// carbon).
pub fn atom_label(atom: &Atom, font_size: f64) -> Option<Label> {
    let degree = atom.bonds.len();

    let text = match &atom.kind {
        AtomKind::Star => "*".to_string(),
        AtomKind::Shortcut(shortcut) => {
            let element: Element = shortcut.into();
            if matches!(element, Element::C) && degree > 0 {
                return None;
            }
            let symbol = shortcut_symbol(shortcut);
            let hydrogens = atom.implicit_hydrogens();
            place_hydrogens(symbol, hydrogens, degree == 0)
        }
        AtomKind::Selection(selection) => {
            let element: Element = selection.into();
            if matches!(element, Element::C) && degree > 0 {
                return None;
            }
            let symbol = selection_symbol(selection);
            let hydrogens = atom.implicit_hydrogens();
            place_hydrogens(symbol, hydrogens, degree == 0)
        }
        AtomKind::Bracket(bracket) => {
            let mut text = String::new();

            if let Some(isotope) = &bracket.isotope {
                text.push_str(&superscript_digits(&isotope.to_string()));
            }

            text.push_str(&bracket.symbol.to_string());

            if let Some(hydrogens) = &bracket.hydrogens {
                let count: u8 = hydrogens.into();
                text.push('H');
                if count > 1 {
                    text.push(subscript_digit(u64::from(count)));
                }
            }

            if let Some(charge) = &bracket.charge {
                text.push_str(&superscript_charge(charge));
            }

            text
        }
    };

    Some(Label::new(text, font_size))
}

fn shortcut_symbol(shortcut: &Shortcut) -> String {
    shortcut.to_string()
}

fn selection_symbol(selection: &Selection) -> String {
    selection.to_string()
}

/// Put hydrogens before or after the element symbol using the conventional
/// hydrogen-first spelling for lone neutral hydrides (H2O, HCl, ...).
fn place_hydrogens(symbol: String, hydrogens: u8, lone: bool) -> String {
    if hydrogens == 0 {
        return symbol;
    }

    let hydrogen_first = lone && matches!(symbol.as_str(), "O" | "S" | "F" | "Cl" | "Br" | "I");
    let hydrogen = hydrogen_group(hydrogens);

    if hydrogen_first {
        format!("{hydrogen}{symbol}")
    } else {
        format!("{symbol}{hydrogen}")
    }
}

fn hydrogen_group(count: u8) -> String {
    let mut text = String::from("H");
    if count > 1 {
        text.push(subscript_digit(u64::from(count)));
    }
    text
}

fn superscript_charge(charge: &Charge) -> String {
    let (magnitude, negative) = match charge {
        Charge::Minus9 => (9, true),
        Charge::Minus8 => (8, true),
        Charge::Minus7 => (7, true),
        Charge::Minus6 => (6, true),
        Charge::Minus5 => (5, true),
        Charge::Minus4 => (4, true),
        Charge::Minus3 => (3, true),
        Charge::Minus2 => (2, true),
        Charge::Minus1 | Charge::Minus => (1, true),
        Charge::Plus9 => (9, false),
        Charge::Plus8 => (8, false),
        Charge::Plus7 => (7, false),
        Charge::Plus6 => (6, false),
        Charge::Plus5 => (5, false),
        Charge::Plus4 => (4, false),
        Charge::Plus3 => (3, false),
        Charge::Plus2 => (2, false),
        Charge::Plus1 | Charge::Plus => (1, false),
    };

    let mut text = String::new();
    if magnitude > 1 {
        text.push(superscript_digit(magnitude));
    }
    text.push(if negative { '\u{207B}' } else { '\u{207A}' });
    text
}

fn subscript_digit(value: u64) -> char {
    char::from_u32(SUBSCRIPT_ZERO + value as u32).unwrap_or('0')
}

fn superscript_digit(value: u64) -> char {
    match value {
        0 => char::from_u32(SUPERSCRIPT_ZERO).expect("superscript zero"),
        1 => char::from_u32(SUPERSCRIPT_ONE).expect("superscript one"),
        2 => char::from_u32(SUPERSCRIPT_TWO).expect("superscript two"),
        3 => char::from_u32(SUPERSCRIPT_THREE).expect("superscript three"),
        _ => char::from_u32(SUPERSCRIPT_ZERO + value as u32).unwrap_or('0'),
    }
}

fn superscript_digits(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| character.to_digit(10))
        .map(|digit| superscript_digit(u64::from(digit)))
        .collect()
}

fn is_script_character(character: char) -> bool {
    let code = character as u32;
    (SUBSCRIPT_ZERO..=SUBSCRIPT_ZERO + 9).contains(&code)
        || (SUPERSCRIPT_ZERO..=SUPERSCRIPT_ZERO + 9).contains(&code)
        || matches!(
            character,
            '\u{00B9}' | '\u{00B2}' | '\u{00B3}' | '\u{207A}' | '\u{207B}'
        )
}

/// Add the glyphs to the SVG text layer and reserve their area in the mask.
pub fn render_atom(atom: &Atom, position: Vec2, font_size: f64, masks: &mut String, texts: &mut String) {
    let Some(label) = atom_label(atom, font_size) else {
        return;
    };

    let width = label.width();
    masks.push_str(&format!(
        r#"<ellipse cx="{:.3}" cy="{:.3}" rx="{:.3}" ry="{:.3}" fill="black" />"#,
        position.x,
        position.y,
        width / 2.0 + 2.0,
        label.mask_height()
    ));

    texts.push_str(&format!(
        r#"<text x="{:.3}" y="{:.3}" text-anchor="middle" dominant-baseline="central" style="font-family:'Helvetica Neue',Helvetica,Arial,sans-serif;font-size:{:.1};fill:white">{}</text>"#,
        position.x, position.y, label.font_size, label.text
    ));
}

/// Width reserved around an atom by its label, for the canvas bounding box.
pub fn label_extent(atom: &Atom, font_size: f64) -> Option<(f64, f64)> {
    atom_label(atom, font_size).map(|label| (label.width() / 2.0, label.mask_height()))
}

#[cfg(test)]
mod tests {
    use balso_core::{Bracket, Element, Isotope, Symbol, VirtualHydrogen};

    use super::*;
    use crate::Bond;

    #[test]
    fn skeletal_carbon_is_hidden() {
        let atom = Atom {
            kind: AtomKind::Shortcut(Shortcut::C),
            bonds: vec![Bond::elided(1)],
        };

        assert_eq!(atom_label(&atom, 16.0), None);
    }

    #[test]
    fn terminal_oxygen_gets_hydrogen() {
        let atom = Atom {
            kind: AtomKind::Shortcut(Shortcut::O),
            bonds: vec![Bond::elided(0)],
        };

        assert_eq!(atom_label(&atom, 16.0).unwrap().text, "OH");
    }

    #[test]
    fn lone_water_is_hydrogen_first() {
        let atom = Atom::shortcut(Shortcut::O, vec![]);

        assert_eq!(atom_label(&atom, 16.0).unwrap().text, "H₂O");
    }

    #[test]
    fn bracket_assembles_isotope_hydrogens_and_charge() {
        let atom = Atom::bracket(
            Bracket {
                symbol: Symbol::Element(Element::N),
                isotope: Some(Isotope::new(13).unwrap()),
                hydrogens: Some(VirtualHydrogen::H4),
                charge: Some(Charge::Plus),
                ..Default::default()
            },
            vec![],
        );

        assert_eq!(atom_label(&atom, 16.0).unwrap().text, "¹³NH₄⁺");
    }
}
