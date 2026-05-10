use super::{util::Vec2, Atom};

fn int2subscript(n: usize) -> char {
    ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉']
        .get(n)
        .cloned()
        .unwrap_or('₀')
}

#[derive(Debug, Clone, Copy)]
pub enum TextDirection {
    Left,
    Middle,
    Right,
}

fn render_text(out: &mut String, x: f64, y: f64, label: &str, dir: TextDirection) {
    match dir {
        TextDirection::Left => {
            let style = "text-anchor:end;dominant-baseline:alphabetic;font-family:sans-serif;font-size:14;fill:white;data-direction:left";
            let reversed_label: String = label.chars().rev().collect();
            out.push_str(&format!(
                r#"<text x="{:.3}" y="{:.3}" dx="0.38em" dy="0.36em" style="{}">{}</text>"#,
                x, y, style, reversed_label
            ));
        }
        TextDirection::Middle => {
            let style =
                "text-anchor:middle;dominant-baseline:alphabetic;font-family:sans-serif;font-size:14;fill:white";
            out.push_str(&format!(
                r#"<text x="{:.3}" y="{:.3}" dy="0.36em" style="{}">{}</text>"#,
                x, y, style, label
            ));
        }
        TextDirection::Right => {
            let style = "text-anchor:start;dominant-baseline:alphabetic;font-family:sans-serif;font-size:14;fill:white";
            out.push_str(&format!(
                r#"<text x="{:.3}" y="{:.3}" dx="-0.38em" dy="0.36em" style="{}">{}</text>"#,
                x, y, style, label
            ));
        }
    }
}

pub fn render_atom(atom: &Atom, pos: Vec2, visible: bool, masks: &mut String, texts: &mut String) {
    if visible {
        masks.push_str(&format!(
            r#"<circle cx="{:.3}" cy="{:.3}" r="10" fill="black" />"#,
            pos.x, pos.y
        ));

        let mut label = atom.kind.text();

        let h = atom.implicit_hydrogens();
        if h > 0 {
            label.push('H');
            if h > 1 {
                label.push(int2subscript(h.into()));
            }
        }

        let dir = if h > 0 {
            TextDirection::Left
        } else {
            TextDirection::Middle
        };

        render_text(texts, pos.x, pos.y, &label, dir);
    }
}
