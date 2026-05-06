use super::{util::Vec2, Atom};

pub fn render_text(atom: &Atom, pos: Vec2, visible: bool, masks: &mut String, texts: &mut String) {
    if visible {
        masks.push_str(&format!(
            r#"<circle cx="{:.3}" cy="{:.3}" r="10" fill="black" />"#,
            pos.x, pos.y
        ));
        texts.push_str(&format!(
            r#"<text x="{:.3}" y="{:.3}" style="text-anchor:middle;dominant-baseline:middle;font-family:sans-serif;font-size:14;fill:white">{}</text>"#,
            pos.x, pos.y, atom.kind
        ));
    }
}
