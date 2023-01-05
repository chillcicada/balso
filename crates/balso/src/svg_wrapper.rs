//! SVG wrapper for generating SVG output.
//!
//! This module provides utilities for creating and manipulating SVG elements.

use nalgebra::Vector2;

/// An SVG element type.
#[derive(Debug, Clone, PartialEq)]
pub enum SvgElement {
    /// A line element.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: String,
        stroke_width: f64,
    },
    /// A path element.
    Path {
        d: String,
        stroke: String,
        stroke_width: f64,
        fill: Option<String>,
    },
    /// A circle element.
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// A text element.
    Text {
        x: f64,
        y: f64,
        content: String,
        fill: String,
        font_size: Option<f64>,
        font_family: Option<String>,
        text_anchor: Option<String>,
        dominant_baseline: Option<String>,
    },
    /// A wedge (triangle) for stereochemistry.
    Wedge {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        fill: String,
    },
    /// A group element.
    Group {
        elements: Vec<SvgElement>,
        transform: Option<String>,
    },
}

impl SvgElement {
    /// Convert to SVG string.
    pub fn to_string(&self) -> String {
        match self {
            SvgElement::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                stroke_width,
            } => format!(
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}" />"##,
                round(*x1),
                round(*y1),
                round(*x2),
                round(*y2),
                stroke,
                round(*stroke_width)
            ),
            SvgElement::Path {
                d,
                stroke,
                stroke_width,
                fill,
            } => {
                if let Some(fill_color) = fill {
                    format!(
                        r##"<path d="{}" stroke="{}" stroke-width="{}" fill="{}" />"##,
                        d, stroke, round(*stroke_width), fill_color
                    )
                } else {
                    format!(
                        r##"<path d="{}" stroke="{}" stroke-width="{}" fill="none" />"##,
                        d, stroke, round(*stroke_width)
                    )
                }
            }
            SvgElement::Circle {
                cx,
                cy,
                r,
                fill,
                stroke,
                stroke_width,
            } => {
                let fill_str = fill
                    .as_ref()
                    .map(|s| format!(r#" fill="{}""#, s))
                    .unwrap_or_default();
                let stroke_str = stroke
                    .as_ref()
                    .map(|s| format!(r#" stroke="{}""#, s))
                    .unwrap_or_default();
                let stroke_width_str = stroke_width
                    .as_ref()
                    .map(|w| format!(r#" stroke-width="{}""#, round(*w)))
                    .unwrap_or_default();
                format!(
                    r##"<circle cx="{}" cy="{}" r="{}"{}{}{} />"##,
                    round(*cx),
                    round(*cy),
                    round(*r),
                    fill_str,
                    stroke_str,
                    stroke_width_str
                )
            }
            SvgElement::Text {
                x,
                y,
                content,
                fill,
                font_size,
                font_family,
                text_anchor,
                dominant_baseline,
            } => {
                let font_size_str = font_size
                    .as_ref()
                    .map(|s| format!(r#" font-size="{}""#, round(*s)))
                    .unwrap_or_default();
                let font_family_str = font_family
                    .as_ref()
                    .map(|s| format!(r#" font-family="{}""#, s))
                    .unwrap_or_default();
                let text_anchor_str = text_anchor
                    .as_ref()
                    .map(|s| format!(r#" text-anchor="{}""#, s))
                    .unwrap_or_default();
                let dominant_baseline_str = dominant_baseline
                    .as_ref()
                    .map(|s| format!(r#" dominant-baseline="{}""#, s))
                    .unwrap_or_default();
                format!(
                    r##"<text x="{}" y="{}" fill="{}"{}{}{}{}>{}</text>"##,
                    round(*x),
                    round(*y),
                    fill,
                    font_size_str,
                    font_family_str,
                    text_anchor_str,
                    dominant_baseline_str,
                    escape_html(content)
                )
            }
            SvgElement::Wedge {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
                fill,
            } => format!(
                r##"<polygon points="{},{} {},{} {},{}" fill="{}" />"##,
                round(*x1),
                round(*y1),
                round(*x2),
                round(*y2),
                round(*x3),
                round(*y3),
                fill
            ),
            SvgElement::Group {
                elements,
                transform,
            } => {
                let transform_str = transform
                    .as_ref()
                    .map(|s| format!(r#" transform="{}""#, s))
                    .unwrap_or_default();
                let content: String = elements.iter().map(|e| e.to_string()).collect();
                format!(r##"<g{}>{}</g>"##, transform_str, content)
            }
        }
    }
}

/// Round a float to 2 decimal places for cleaner output.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Escape HTML special characters in text content.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Builder for SVG documents.
#[derive(Debug, Clone)]
pub struct SvgWrapper {
    elements: Vec<SvgElement>,
    width: f64,
    height: f64,
    view_box: Option<(f64, f64, f64, f64)>,
    padding: f64,
}

impl SvgWrapper {
    /// Create a new SVG wrapper.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            width: 0.0,
            height: 0.0,
            view_box: None,
            padding: 20.0,
        }
    }

    /// Create a new SVG wrapper with custom padding.
    pub fn with_padding(padding: f64) -> Self {
        Self {
            elements: Vec::new(),
            width: 0.0,
            height: 0.0,
            view_box: None,
            padding,
        }
    }

    /// Set the SVG dimensions.
    pub fn set_dimensions(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    /// Set the viewBox.
    pub fn set_viewbox(&mut self, min_x: f64, min_y: f64, width: f64, height: f64) {
        self.view_box = Some((min_x, min_y, width, height));
    }

    /// Add an element to the SVG.
    pub fn add_element(&mut self, element: SvgElement) {
        self.elements.push(element);
    }

    /// Add a line.
    pub fn add_line(
        &mut self,
        a: Vector2<f64>,
        b: Vector2<f64>,
        stroke: String,
        stroke_width: f64,
    ) {
        self.elements.push(SvgElement::Line {
            x1: a.x,
            y1: a.y,
            x2: b.x,
            y2: b.y,
            stroke,
            stroke_width,
        });
    }

    /// Add a path.
    pub fn add_path(
        &mut self,
        d: String,
        stroke: String,
        stroke_width: f64,
        fill: Option<String>,
    ) {
        self.elements.push(SvgElement::Path {
            d,
            stroke,
            stroke_width,
            fill,
        });
    }

    /// Add a circle.
    pub fn add_circle(
        &mut self,
        center: Vector2<f64>,
        r: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    ) {
        self.elements.push(SvgElement::Circle {
            cx: center.x,
            cy: center.y,
            r,
            fill,
            stroke,
            stroke_width,
        });
    }

    /// Add text.
    pub fn add_text(
        &mut self,
        pos: Vector2<f64>,
        content: String,
        fill: String,
        font_size: Option<f64>,
    ) {
        self.elements.push(SvgElement::Text {
            x: pos.x,
            y: pos.y,
            content,
            fill,
            font_size,
            font_family: None,
            text_anchor: None,
            dominant_baseline: None,
        });
    }

    /// Add text with full options.
    pub fn add_text_full(
        &mut self,
        pos: Vector2<f64>,
        content: String,
        fill: String,
        font_size: f64,
        font_family: String,
        text_anchor: String,
        dominant_baseline: String,
    ) {
        self.elements.push(SvgElement::Text {
            x: pos.x,
            y: pos.y,
            content,
            fill,
            font_size: Some(font_size),
            font_family: Some(font_family),
            text_anchor: Some(text_anchor),
            dominant_baseline: Some(dominant_baseline),
        });
    }

    /// Add a wedge (solid triangle for up bonds).
    pub fn add_wedge(&mut self, a: Vector2<f64>, b: Vector2<f64>, width: f64, fill: String) {
        // Calculate perpendicular vector for wedge width
        let direction = b - a;
        let len = direction.magnitude();
        if len == 0.0 {
            return;
        }

        let normalized = direction / len;
        let perp = Vector2::new(-normalized.y, normalized.x);

        // Calculate wedge vertices (narrow at a, wide at b)
        let a_point = a;
        let b1 = b + perp * (width / 2.0);
        let b2 = b - perp * (width / 2.0);

        self.elements.push(SvgElement::Wedge {
            x1: a_point.x,
            y1: a_point.y,
            x2: b1.x,
            y2: b1.y,
            x3: b2.x,
            y3: b2.y,
            fill,
        });
    }

    /// Add a hashed wedge (dashed triangle for down bonds).
    pub fn add_hashed_wedge(
        &mut self,
        a: Vector2<f64>,
        b: Vector2<f64>,
        width: f64,
        fill: String,
        stroke: String,
        stroke_width: f64,
    ) {
        // For hashed wedge, we draw parallel lines
        let direction = b - a;
        let len = direction.magnitude();
        if len == 0.0 {
            return;
        }

        let normalized = direction / len;
        let perp = Vector2::new(-normalized.y, normalized.x);
        let num_lines = 5;

        for i in 0..num_lines {
            let t = (i as f64 + 1.0) / (num_lines as f64 + 1.0);
            let offset = ((i as f64 + 1.0) / (num_lines as f64)) - 0.5;

            let mid = a + direction * t;
            let half_width = width * t / 2.0;

            let p1 = mid + perp * half_width;
            let p2 = mid - perp * half_width;

            self.elements.push(SvgElement::Line {
                x1: p1.x,
                y1: p1.y,
                x2: p2.x,
                y2: p2.y,
                stroke: stroke.clone(),
                stroke_width,
            });
        }
    }

    /// Add an aromaticity circle (dashed circle for aromatic rings).
    pub fn add_aromatic_circle(
        &mut self,
        center: Vector2<f64>,
        radius: f64,
        stroke: String,
        stroke_width: f64,
    ) {
        // For SVG, we use a circle with dashed stroke
        let d = format!(
            "M {} {} A {} {} 0 1 1 {} {} A {} {} 0 1 1 {} {}",
            center.x + radius,
            center.y,
            radius,
            radius,
            center.x - radius,
            center.y,
            radius,
            radius,
            center.x + radius,
            center.y
        );

        self.elements.push(SvgElement::Path {
            d,
            stroke,
            stroke_width,
            fill: None,
        });
    }

    /// Generate the final SVG string.
    pub fn to_string(&self) -> String {
        let view_box_str = if let Some((x, y, w, h)) = self.view_box {
            format!(r#" viewBox="{} {} {} {}""#, round(x), round(y), round(w), round(h))
        } else {
            String::new()
        };

        let elements_str: String = self.elements.iter().map(|e| e.to_string()).collect();

        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}"{}>{}</svg>"#,
            round(self.width),
            round(self.height),
            view_box_str,
            elements_str
        )
    }

    /// Get elements count.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Default for SvgWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for geometry.
pub mod geometry {
    use nalgebra::Vector2;

    /// Calculate the midpoint between two points.
    pub fn midpoint(a: &Vector2<f64>, b: &Vector2<f64>) -> Vector2<f64> {
        (a + b) / 2.0
    }

    /// Calculate the normal vector to a line segment.
    pub fn normal(a: &Vector2<f64>, b: &Vector2<f64>) -> Vector2<f64> {
        let direction = b - a;
        let len = direction.magnitude();
        if len == 0.0 {
            return Vector2::new(0.0, 0.0);
        }
        let normalized = direction / len;
        Vector2::new(-normalized.y, normalized.x)
    }

    /// Calculate perpendicular normals for offsetting double bonds.
    pub fn double_bond_normals(a: &Vector2<f64>, b: &Vector2<f64>, spacing: f64) -> [Vector2<f64>; 2] {
        let dir = b - a;
        let len = dir.magnitude();
        if len == 0.0 {
            return [Vector2::zeros(); 2];
        }
        let normalized = dir / len;
        let perp = Vector2::new(-normalized.y, normalized.x);
        [perp * spacing, perp * -spacing]
    }

    /// Shorten a line segment from both ends.
    pub fn shorten_line(
        a: Vector2<f64>,
        b: Vector2<f64>,
        shorten_from_start: f64,
        shorten_from_end: f64,
    ) -> (Vector2<f64>, Vector2<f64>) {
        let direction = b - a;
        let len = direction.magnitude();
        if len == 0.0 {
            return (a, b);
        }

        let normalized = direction / len;
        let new_a = a + normalized * shorten_from_start;
        let new_b = a + normalized * (len - shorten_from_end);
        (new_a, new_b)
    }

    /// Get the direction from a point to another (unit vector).
    pub fn direction(a: &Vector2<f64>, b: &Vector2<f64>) -> Vector2<f64> {
        let dir = b - a;
        let len = dir.magnitude();
        if len == 0.0 {
            return Vector2::new(1.0, 0.0);
        }
        dir / len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_element() {
        let line = SvgElement::Line {
            x1: 10.0,
            y1: 20.0,
            x2: 30.0,
            y2: 40.0,
            stroke: "#333".to_string(),
            stroke_width: 2.0,
        };
        let s = line.to_string();
        assert!(s.contains("<line"));
        assert!(s.contains("x1=\"10\""));
        assert!(s.contains("stroke=\"#333\""));
    }

    #[test]
    fn test_circle_element() {
        let circle = SvgElement::Circle {
            cx: 100.0,
            cy: 100.0,
            r: 50.0,
            fill: Some("#ff0000".to_string()),
            stroke: None,
            stroke_width: None,
        };
        let s = circle.to_string();
        assert!(s.contains("<circle"));
        assert!(s.contains("cx=\"100\""));
        assert!(s.contains("fill=\"#ff0000\""));
    }

    #[test]
    fn test_text_element() {
        let text = SvgElement::Text {
            x: 50.0,
            y: 50.0,
            content: "C".to_string(),
            fill: "#333".to_string(),
            font_size: Some(14.0),
            font_family: Some("Arial".to_string()),
            text_anchor: Some("middle".to_string()),
            dominant_baseline: Some("middle".to_string()),
        };
        let s = text.to_string();
        assert!(s.contains("<text"));
        assert!(s.contains(">C</text>"));
    }

    #[test]
    fn test_svg_wrapper() {
        let mut svg = SvgWrapper::new();
        svg.set_dimensions(200.0, 200.0);
        svg.add_element(SvgElement::Circle {
            cx: 100.0,
            cy: 100.0,
            r: 50.0,
            fill: Some("#ff0000".to_string()),
            stroke: None,
            stroke_width: None,
        });

        let s = svg.to_string();
        assert!(s.contains("<svg"));
        assert!(s.contains("width=\"200\""));
        assert!(s.contains("height=\"200\""));
    }

    #[test]
    fn test_geometry_midpoint() {
        use nalgebra::Vector2;
        let a = Vector2::new(0.0, 0.0);
        let b = Vector2::new(4.0, 4.0);
        let mid = geometry::midpoint(&a, &b);
        assert_eq!(mid.x, 2.0);
        assert_eq!(mid.y, 2.0);
    }

    #[test]
    fn test_geometry_normal() {
        use nalgebra::Vector2;
        let a = Vector2::new(0.0, 0.0);
        let b = Vector2::new(4.0, 0.0); // Horizontal line
        let normal = geometry::normal(&a, &b);
        // Should be vertical (0, 1) or (0, -1)
        assert!((normal.x.abs()) < 1e-10);
        assert!((normal.y.abs() - 1.0).abs() < 1e-10 || (normal.y.abs() + 1.0).abs() < 1e-10);
    }
}
