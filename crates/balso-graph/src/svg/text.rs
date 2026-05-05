use super::{util::Vec2, Atom};

pub fn render_text(atom: &Atom, pos: Vec2, masks: &mut String, texts: &mut String) {
    if !atom.kind.is_carbon() {
        masks.push_str(&format!(
            r#"<circle cx="{}" cy="{}" r="10" fill="black" />"#,
            pos.x, pos.y
        ));
        texts.push_str(&format!(
            r#"<text x="{}" y="{}" style="text-anchor:middle;dominant-baseline:middle;font-family:sans-serif;font-size:14;fill:white">{}</text>"#,
            pos.x, pos.y, atom.kind
        ));
    }
}
