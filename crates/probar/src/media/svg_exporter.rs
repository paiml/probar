//! SVG Screenshot Generation (Feature 3)
//!
//! Vector-based screenshots for resolution-independent documentation.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application:
//! - **Poka-Yoke**: Type-safe SVG generation prevents malformed output
//! - **Muda**: Efficient string building minimizes allocations

use crate::driver::Screenshot;
use crate::media::png_exporter::{Annotation, AnnotationKind};
use crate::result::{ProbarError, ProbarResult};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;

/// SVG compression options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SvgCompression {
    /// No compression, human-readable output
    #[default]
    None,
    /// Minified output (no whitespace)
    Minified,
}

/// Configuration for SVG export
#[derive(Debug, Clone)]
pub struct SvgConfig {
    /// Viewbox dimensions (width, height)
    pub viewbox: (u32, u32),
    /// Preserve aspect ratio
    pub preserve_aspect_ratio: bool,
    /// Embed fonts as base64
    pub embed_fonts: bool,
    /// Output compression level
    pub compression: SvgCompression,
    /// Include XML declaration
    pub include_xml_declaration: bool,
    /// Title for accessibility
    pub title: Option<String>,
    /// Description for accessibility
    pub description: Option<String>,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self {
            viewbox: (800, 600),
            preserve_aspect_ratio: true,
            embed_fonts: false,
            compression: SvgCompression::None,
            include_xml_declaration: true,
            title: None,
            description: None,
        }
    }
}

impl SvgConfig {
    /// Create a new SVG config with viewbox dimensions
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            viewbox: (width, height),
            preserve_aspect_ratio: true,
            embed_fonts: false,
            compression: SvgCompression::None,
            include_xml_declaration: true,
            title: None,
            description: None,
        }
    }

    /// Set viewbox dimensions
    #[must_use]
    pub const fn with_viewbox(mut self, width: u32, height: u32) -> Self {
        self.viewbox = (width, height);
        self
    }

    /// Set preserve aspect ratio
    #[must_use]
    pub const fn with_preserve_aspect_ratio(mut self, preserve: bool) -> Self {
        self.preserve_aspect_ratio = preserve;
        self
    }

    /// Set compression level
    #[must_use]
    pub const fn with_compression(mut self, compression: SvgCompression) -> Self {
        self.compression = compression;
        self
    }

    /// Set XML declaration inclusion
    #[must_use]
    pub const fn with_xml_declaration(mut self, include: bool) -> Self {
        self.include_xml_declaration = include;
        self
    }

    /// Set title
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set description
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// SVG exporter for creating vector screenshots
#[derive(Debug, Clone)]
pub struct SvgExporter {
    config: SvgConfig,
}

impl Default for SvgExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl SvgExporter {
    /// Create a new SVG exporter with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SvgConfig::default(),
        }
    }

    /// Create a new SVG exporter with custom config
    #[must_use]
    pub const fn with_config(config: SvgConfig) -> Self {
        Self { config }
    }

    /// Get the current config
    #[must_use]
    pub const fn config(&self) -> &SvgConfig {
        &self.config
    }

    /// Export a screenshot as embedded image SVG
    ///
    /// This embeds the raster image as a base64-encoded data URI within the SVG.
    /// The SVG wrapper provides scalability and annotation support.
    ///
    /// # Errors
    ///
    /// Returns error if screenshot data is invalid
    pub fn from_screenshot(&self, screenshot: &Screenshot) -> ProbarResult<String> {
        self.from_screenshot_with_annotations(screenshot, &[])
    }

    /// Export a screenshot with annotations as SVG
    ///
    /// # Errors
    ///
    /// Returns error if screenshot data is invalid
    pub fn from_screenshot_with_annotations(
        &self,
        screenshot: &Screenshot,
        annotations: &[Annotation],
    ) -> ProbarResult<String> {
        let mut svg = String::with_capacity(screenshot.data.len() * 2);
        let newline = self.newline();
        let indent = self.indent();

        // XML declaration
        if self.config.include_xml_declaration {
            svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
            svg.push_str(newline);
        }

        // SVG root element
        let (width, height) = self.config.viewbox;
        let preserve_aspect = if self.config.preserve_aspect_ratio {
            "xMidYMid meet"
        } else {
            "none"
        };

        write!(
            svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
             width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" \
             preserveAspectRatio=\"{preserve_aspect}\">"
        )
        .map_err(|e| ProbarError::ImageProcessing {
            message: e.to_string(),
        })?;
        svg.push_str(newline);

        // Title for accessibility
        if let Some(ref title) = self.config.title {
            write!(svg, "{indent}<title>{}</title>", escape_xml(title)).map_err(|e| {
                ProbarError::ImageProcessing {
                    message: e.to_string(),
                }
            })?;
            svg.push_str(newline);
        }

        // Description for accessibility
        if let Some(ref desc) = self.config.description {
            write!(svg, "{indent}<desc>{}</desc>", escape_xml(desc)).map_err(|e| {
                ProbarError::ImageProcessing {
                    message: e.to_string(),
                }
            })?;
            svg.push_str(newline);
        }

        // Embedded image
        let base64_data = base64_encode(&screenshot.data);
        write!(
            svg,
            "{indent}<image x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" \
             xlink:href=\"data:image/png;base64,{base64_data}\"/>"
        )
        .map_err(|e| ProbarError::ImageProcessing {
            message: e.to_string(),
        })?;
        svg.push_str(newline);

        // Annotations group
        if !annotations.is_empty() {
            write!(svg, "{indent}<g id=\"annotations\">").map_err(|e| {
                ProbarError::ImageProcessing {
                    message: e.to_string(),
                }
            })?;
            svg.push_str(newline);

            for annotation in annotations {
                self.render_annotation(&mut svg, annotation, &format!("{indent}{indent}"))?;
            }

            write!(svg, "{indent}</g>").map_err(|e| ProbarError::ImageProcessing {
                message: e.to_string(),
            })?;
            svg.push_str(newline);
        }

        // Close SVG
        svg.push_str("</svg>");
        svg.push_str(newline);

        Ok(svg)
    }

    /// Create an SVG from raw shapes (no raster image)
    ///
    /// # Errors
    ///
    /// Returns error if rendering fails
    pub fn from_shapes(&self, shapes: &[SvgShape]) -> ProbarResult<String> {
        let mut svg = String::with_capacity(4096);
        let newline = self.newline();
        let indent = self.indent();

        // XML declaration
        if self.config.include_xml_declaration {
            svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
            svg.push_str(newline);
        }

        // SVG root element
        let (width, height) = self.config.viewbox;
        let preserve_aspect = if self.config.preserve_aspect_ratio {
            "xMidYMid meet"
        } else {
            "none"
        };

        write!(
            svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" \
             width=\"{width}\" height=\"{height}\" \
             viewBox=\"0 0 {width} {height}\" \
             preserveAspectRatio=\"{preserve_aspect}\">"
        )
        .map_err(|e| ProbarError::ImageProcessing {
            message: e.to_string(),
        })?;
        svg.push_str(newline);

        // Title
        if let Some(ref title) = self.config.title {
            write!(svg, "{indent}<title>{}</title>", escape_xml(title)).map_err(|e| {
                ProbarError::ImageProcessing {
                    message: e.to_string(),
                }
            })?;
            svg.push_str(newline);
        }

        // Description
        if let Some(ref desc) = self.config.description {
            write!(svg, "{indent}<desc>{}</desc>", escape_xml(desc)).map_err(|e| {
                ProbarError::ImageProcessing {
                    message: e.to_string(),
                }
            })?;
            svg.push_str(newline);
        }

        // Render shapes
        for shape in shapes {
            self.render_shape(&mut svg, shape, indent)?;
            svg.push_str(newline);
        }

        // Close SVG
        svg.push_str("</svg>");
        svg.push_str(newline);

        Ok(svg)
    }

    /// Save SVG to file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be written
    pub fn save(&self, svg_content: &str, path: &Path) -> ProbarResult<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(svg_content.as_bytes())?;
        Ok(())
    }

    /// Get newline based on compression setting
    fn newline(&self) -> &'static str {
        match self.config.compression {
            SvgCompression::None => "\n",
            SvgCompression::Minified => "",
        }
    }

    /// Get indent based on compression setting
    fn indent(&self) -> &'static str {
        match self.config.compression {
            SvgCompression::None => "  ",
            SvgCompression::Minified => "",
        }
    }

    /// Render an annotation to SVG
    fn render_annotation(
        &self,
        svg: &mut String,
        annotation: &Annotation,
        indent: &str,
    ) -> ProbarResult<()> {
        let newline = self.newline();
        let color = color_to_svg(&annotation.color);

        match annotation.kind {
            AnnotationKind::Rectangle => {
                write!(
                    svg,
                    "{indent}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                     fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>",
                    annotation.x, annotation.y, annotation.width, annotation.height
                )
                .map_err(|e| ProbarError::ImageProcessing {
                    message: e.to_string(),
                })?;
            }
            AnnotationKind::FilledRectangle => {
                write!(
                    svg,
                    "{indent}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\"/>",
                    annotation.x, annotation.y, annotation.width, annotation.height
                )
                .map_err(|e| ProbarError::ImageProcessing {
                    message: e.to_string(),
                })?;
            }
            AnnotationKind::Circle => {
                // Use width as diameter for circle
                let r = annotation.width / 2;
                let cx = annotation.x + r;
                let cy = annotation.y + r;
                write!(
                    svg,
                    "{indent}<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" \
                     fill=\"none\" stroke=\"{color}\" stroke-width=\"2\"/>"
                )
                .map_err(|e| ProbarError::ImageProcessing {
                    message: e.to_string(),
                })?;
            }
            AnnotationKind::Arrow => {
                // Arrow from (x, y) to (x + width, y + height)
                let x1 = annotation.x;
                let y1 = annotation.y;
                let x2 = annotation.x + annotation.width;
                let y2 = annotation.y + annotation.height;
                write!(
                    svg,
                    "{indent}<defs>{newline}\
                     {indent}  <marker id=\"arrowhead-{}\" markerWidth=\"10\" markerHeight=\"7\" \
                     refX=\"9\" refY=\"3.5\" orient=\"auto\">{newline}\
                     {indent}    <polygon points=\"0 0, 10 3.5, 0 7\" fill=\"{color}\"/>{newline}\
                     {indent}  </marker>{newline}\
                     {indent}</defs>{newline}\
                     {indent}<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" \
                     stroke=\"{color}\" stroke-width=\"2\" marker-end=\"url(#arrowhead-{})\"/>",
                    annotation.x, annotation.x
                )
                .map_err(|e| ProbarError::ImageProcessing {
                    message: e.to_string(),
                })?;
            }
            AnnotationKind::Highlight => {
                // Semi-transparent highlight
                write!(
                    svg,
                    "{indent}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                     fill=\"{color}\" fill-opacity=\"0.3\"/>",
                    annotation.x, annotation.y, annotation.width, annotation.height
                )
                .map_err(|e| ProbarError::ImageProcessing {
                    message: e.to_string(),
                })?;
            }
        }

        // Add label if present
        if let Some(ref label) = annotation.label {
            svg.push_str(newline);
            write!(
                svg,
                "{indent}<text x=\"{}\" y=\"{}\" fill=\"{color}\" font-size=\"12\">{}</text>",
                annotation.x,
                annotation.y.saturating_sub(5),
                escape_xml(label)
            )
            .map_err(|e| ProbarError::ImageProcessing {
                message: e.to_string(),
            })?;
        }

        svg.push_str(newline);
        Ok(())
    }

    /// Render a shape to SVG
    fn render_shape(&self, svg: &mut String, shape: &SvgShape, indent: &str) -> ProbarResult<()> {
        // Helper macro to convert fmt::Error to ProbarError
        macro_rules! w {
            ($($arg:tt)*) => {
                write!($($arg)*).map_err(|e| ProbarError::ImageProcessing { message: e.to_string() })
            };
        }

        match shape {
            SvgShape::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
                stroke_width,
                rx,
                ry,
            } => {
                w!(
                    svg,
                    "{indent}<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\""
                )?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                if let Some(rx) = rx {
                    w!(svg, " rx=\"{rx}\"")?;
                }
                if let Some(ry) = ry {
                    w!(svg, " ry=\"{ry}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Circle {
                cx,
                cy,
                r,
                fill,
                stroke,
                stroke_width,
            } => {
                w!(svg, "{indent}<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\"")?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Ellipse {
                cx,
                cy,
                rx,
                ry,
                fill,
                stroke,
                stroke_width,
            } => {
                w!(
                    svg,
                    "{indent}<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\""
                )?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                stroke_width,
            } => {
                w!(
                    svg,
                    "{indent}<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\""
                )?;
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Polyline {
                points,
                stroke,
                stroke_width,
                fill,
            } => {
                let points_str: String = points
                    .iter()
                    .map(|(x, y)| format!("{x},{y}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                w!(svg, "{indent}<polyline points=\"{points_str}\"")?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                } else {
                    w!(svg, " fill=\"none\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Polygon {
                points,
                fill,
                stroke,
                stroke_width,
            } => {
                let points_str: String = points
                    .iter()
                    .map(|(x, y)| format!("{x},{y}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                w!(svg, "{indent}<polygon points=\"{points_str}\"")?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Path {
                d,
                fill,
                stroke,
                stroke_width,
            } => {
                w!(svg, "{indent}<path d=\"{d}\"")?;
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(stroke) = stroke {
                    w!(svg, " stroke=\"{stroke}\"")?;
                }
                if let Some(sw) = stroke_width {
                    w!(svg, " stroke-width=\"{sw}\"")?;
                }
                w!(svg, "/>")?;
            }
            SvgShape::Text {
                x,
                y,
                content,
                font_size,
                fill,
                font_family,
            } => {
                w!(svg, "{indent}<text x=\"{x}\" y=\"{y}\"")?;
                if let Some(size) = font_size {
                    w!(svg, " font-size=\"{size}\"")?;
                }
                if let Some(fill) = fill {
                    w!(svg, " fill=\"{fill}\"")?;
                }
                if let Some(family) = font_family {
                    w!(svg, " font-family=\"{family}\"")?;
                }
                w!(svg, ">{}</text>", escape_xml(content))?;
            }
            SvgShape::Group { id, children } => {
                if let Some(id) = id {
                    w!(svg, "{indent}<g id=\"{id}\">")?;
                } else {
                    w!(svg, "{indent}<g>")?;
                }
                let newline = self.newline();
                svg.push_str(newline);
                let child_indent = format!("{indent}  ");
                for child in children {
                    self.render_shape(svg, child, &child_indent)?;
                    svg.push_str(newline);
                }
                w!(svg, "{indent}</g>")?;
            }
        }
        Ok(())
    }
}

/// SVG shape primitives
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum SvgShape {
    /// Rectangle
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
        rx: Option<f64>,
        ry: Option<f64>,
    },
    /// Circle
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// Ellipse
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// Line
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// Polyline (open path)
    Polyline {
        points: Vec<(f64, f64)>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
        fill: Option<String>,
    },
    /// Polygon (closed path)
    Polygon {
        points: Vec<(f64, f64)>,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// Path with SVG path data
    Path {
        d: String,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: Option<f64>,
    },
    /// Text
    Text {
        x: f64,
        y: f64,
        content: String,
        font_size: Option<f64>,
        fill: Option<String>,
        font_family: Option<String>,
    },
    /// Group of shapes
    Group {
        id: Option<String>,
        children: Vec<SvgShape>,
    },
}

impl SvgShape {
    /// Create a rectangle
    #[must_use]
    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::Rect {
            x,
            y,
            width,
            height,
            fill: None,
            stroke: None,
            stroke_width: None,
            rx: None,
            ry: None,
        }
    }

    /// Create a circle
    #[must_use]
    pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
        Self::Circle {
            cx,
            cy,
            r,
            fill: None,
            stroke: None,
            stroke_width: None,
        }
    }

    /// Create a line
    #[must_use]
    pub fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self::Line {
            x1,
            y1,
            x2,
            y2,
            stroke: None,
            stroke_width: None,
        }
    }

    /// Create text
    #[must_use]
    pub fn text(x: f64, y: f64, content: impl Into<String>) -> Self {
        Self::Text {
            x,
            y,
            content: content.into(),
            font_size: None,
            fill: None,
            font_family: None,
        }
    }

    /// Set fill color
    #[must_use]
    pub fn with_fill(mut self, fill: impl Into<String>) -> Self {
        match &mut self {
            Self::Rect { fill: f, .. }
            | Self::Circle { fill: f, .. }
            | Self::Ellipse { fill: f, .. }
            | Self::Polygon { fill: f, .. }
            | Self::Path { fill: f, .. }
            | Self::Polyline { fill: f, .. } => *f = Some(fill.into()),
            Self::Text { fill: f, .. } => *f = Some(fill.into()),
            Self::Line { .. } | Self::Group { .. } => {}
        }
        self
    }

    /// Set stroke color
    #[must_use]
    pub fn with_stroke(mut self, stroke: impl Into<String>) -> Self {
        match &mut self {
            Self::Rect { stroke: s, .. }
            | Self::Circle { stroke: s, .. }
            | Self::Ellipse { stroke: s, .. }
            | Self::Line { stroke: s, .. }
            | Self::Polyline { stroke: s, .. }
            | Self::Polygon { stroke: s, .. }
            | Self::Path { stroke: s, .. } => *s = Some(stroke.into()),
            Self::Text { .. } | Self::Group { .. } => {}
        }
        self
    }

    /// Set stroke width
    #[must_use]
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        match &mut self {
            Self::Rect {
                stroke_width: sw, ..
            }
            | Self::Circle {
                stroke_width: sw, ..
            }
            | Self::Ellipse {
                stroke_width: sw, ..
            }
            | Self::Line {
                stroke_width: sw, ..
            }
            | Self::Polyline {
                stroke_width: sw, ..
            }
            | Self::Polygon {
                stroke_width: sw, ..
            }
            | Self::Path {
                stroke_width: sw, ..
            } => *sw = Some(width),
            Self::Text { .. } | Self::Group { .. } => {}
        }
        self
    }
}

/// Convert annotation color to SVG color string
fn color_to_svg(color: &[u8; 4]) -> String {
    format!(
        "rgba({},{},{},{})",
        color[0],
        color[1],
        color[2],
        f64::from(color[3]) / 255.0
    )
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Base64 encode data
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        // Safety: chunks(3) on non-empty slice produces chunks of 1, 2, or 3 elements
        // The 0 case handles the (impossible) edge case to satisfy exhaustiveness
        let n = match chunk.len() {
            3 => (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]),
            2 => (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8),
            1 => u32::from(chunk[0]) << 16,
            0 => continue, // Empty chunk - skip (cannot happen with chunks(3) on non-empty data)
            _ => (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]),
        };

        result.push(char::from(ALPHABET[(n >> 18) as usize & 0x3F]));
        result.push(char::from(ALPHABET[(n >> 12) as usize & 0x3F]));

        if chunk.len() > 1 {
            result.push(char::from(ALPHABET[(n >> 6) as usize & 0x3F]));
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(char::from(ALPHABET[n as usize & 0x3F]));
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn test_screenshot() -> Screenshot {
        Screenshot {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes
            width: 100,
            height: 100,
            device_pixel_ratio: 1.0,
            timestamp: SystemTime::now(),
        }
    }

    mod svg_config_tests {
        use super::*;

        #[test]
        fn test_default_config() {
            let config = SvgConfig::default();
            assert_eq!(config.viewbox, (800, 600));
            assert!(config.preserve_aspect_ratio);
            assert!(!config.embed_fonts);
            assert_eq!(config.compression, SvgCompression::None);
            assert!(config.include_xml_declaration);
        }

        #[test]
        fn test_new_with_dimensions() {
            let config = SvgConfig::new(1920, 1080);
            assert_eq!(config.viewbox, (1920, 1080));
        }

        #[test]
        fn test_builder_chain() {
            let config = SvgConfig::new(800, 600)
                .with_viewbox(1024, 768)
                .with_preserve_aspect_ratio(false)
                .with_compression(SvgCompression::Minified)
                .with_xml_declaration(false)
                .with_title("Test Screenshot")
                .with_description("A test description");

            assert_eq!(config.viewbox, (1024, 768));
            assert!(!config.preserve_aspect_ratio);
            assert_eq!(config.compression, SvgCompression::Minified);
            assert!(!config.include_xml_declaration);
            assert_eq!(config.title, Some("Test Screenshot".to_string()));
            assert_eq!(config.description, Some("A test description".to_string()));
        }
    }

    mod svg_exporter_tests {
        use super::*;

        #[test]
        fn test_default_exporter() {
            let exporter = SvgExporter::new();
            assert_eq!(exporter.config().viewbox, (800, 600));
        }

        #[test]
        fn test_exporter_with_config() {
            let config = SvgConfig::new(1920, 1080);
            let exporter = SvgExporter::with_config(config);
            assert_eq!(exporter.config().viewbox, (1920, 1080));
        }

        #[test]
        fn test_from_screenshot() {
            let screenshot = test_screenshot();
            let exporter = SvgExporter::with_config(SvgConfig::new(100, 100));

            let svg = exporter.from_screenshot(&screenshot).unwrap();

            assert!(svg.contains("<?xml version=\"1.0\""));
            assert!(svg.contains("<svg"));
            assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
            assert!(svg.contains("<image"));
            assert!(svg.contains("data:image/png;base64,"));
            assert!(svg.contains("</svg>"));
        }

        #[test]
        fn test_from_screenshot_with_annotations() {
            let screenshot = test_screenshot();
            let annotations = vec![Annotation::rectangle(10, 10, 50, 30)];
            let exporter = SvgExporter::with_config(SvgConfig::new(100, 100));

            let svg = exporter
                .from_screenshot_with_annotations(&screenshot, &annotations)
                .unwrap();

            assert!(svg.contains("<g id=\"annotations\">"));
            assert!(svg.contains("<rect"));
        }

        #[test]
        fn test_minified_output() {
            let screenshot = test_screenshot();
            let config = SvgConfig::new(100, 100)
                .with_compression(SvgCompression::Minified)
                .with_xml_declaration(false);
            let exporter = SvgExporter::with_config(config);

            let svg = exporter.from_screenshot(&screenshot).unwrap();

            // Minified should have no newlines within
            assert!(!svg.contains("\n  "));
        }

        #[test]
        fn test_with_title_and_description() {
            let screenshot = test_screenshot();
            let config = SvgConfig::new(100, 100)
                .with_title("My Screenshot")
                .with_description("Test description");
            let exporter = SvgExporter::with_config(config);

            let svg = exporter.from_screenshot(&screenshot).unwrap();

            assert!(svg.contains("<title>My Screenshot</title>"));
            assert!(svg.contains("<desc>Test description</desc>"));
        }
    }

    mod svg_shape_tests {
        use super::*;

        #[test]
        fn test_from_shapes() {
            let shapes = vec![
                SvgShape::rect(10.0, 10.0, 100.0, 50.0)
                    .with_fill("blue")
                    .with_stroke("black")
                    .with_stroke_width(2.0),
                SvgShape::circle(150.0, 50.0, 25.0).with_fill("red"),
                SvgShape::line(200.0, 10.0, 300.0, 60.0)
                    .with_stroke("green")
                    .with_stroke_width(3.0),
                SvgShape::text(10.0, 100.0, "Hello SVG").with_fill("black"),
            ];

            let exporter = SvgExporter::with_config(SvgConfig::new(400, 150));
            let svg = exporter.from_shapes(&shapes).unwrap();

            assert!(svg.contains("<rect"));
            assert!(svg.contains("fill=\"blue\""));
            assert!(svg.contains("<circle"));
            assert!(svg.contains("<line"));
            assert!(svg.contains("<text"));
            assert!(svg.contains("Hello SVG"));
        }

        #[test]
        fn test_shape_builders() {
            let rect = SvgShape::rect(0.0, 0.0, 100.0, 100.0);
            assert!(matches!(rect, SvgShape::Rect { x: _, y: _, .. }));

            let circle = SvgShape::circle(50.0, 50.0, 25.0);
            assert!(matches!(
                circle,
                SvgShape::Circle {
                    cx: _,
                    cy: _,
                    r: _,
                    ..
                }
            ));

            let line = SvgShape::line(0.0, 0.0, 100.0, 100.0);
            assert!(matches!(
                line,
                SvgShape::Line {
                    x1: _,
                    y1: _,
                    x2: _,
                    y2: _,
                    ..
                }
            ));

            let text = SvgShape::text(10.0, 20.0, "Test");
            assert!(matches!(text, SvgShape::Text { x: _, y: _, .. }));
        }

        #[test]
        fn test_group_shapes() {
            let shapes = vec![SvgShape::Group {
                id: Some("my-group".to_string()),
                children: vec![
                    SvgShape::rect(0.0, 0.0, 50.0, 50.0).with_fill("blue"),
                    SvgShape::circle(25.0, 25.0, 10.0).with_fill("red"),
                ],
            }];

            let exporter = SvgExporter::with_config(SvgConfig::new(100, 100));
            let svg = exporter.from_shapes(&shapes).unwrap();

            assert!(svg.contains("<g id=\"my-group\">"));
            assert!(svg.contains("<rect"));
            assert!(svg.contains("<circle"));
            assert!(svg.contains("</g>"));
        }
    }

    mod annotation_tests {
        use super::*;

        #[test]
        fn test_all_annotation_types() {
            let screenshot = test_screenshot();
            let annotations = vec![
                Annotation::rectangle(10, 10, 50, 30),
                Annotation::highlight(60, 10, 50, 30),
                Annotation::circle(120, 25, 15),
                Annotation::arrow(150, 25, 50, 0),
                Annotation::filled_rectangle(10, 60, 50, 30),
            ];

            let exporter = SvgExporter::with_config(SvgConfig::new(200, 200));
            let svg = exporter
                .from_screenshot_with_annotations(&screenshot, &annotations)
                .unwrap();

            // Check all annotation types are rendered
            assert!(svg.contains("<rect")); // Rectangle, FilledRectangle, and Highlight
            assert!(svg.contains("<circle"));
            assert!(svg.contains("<line")); // Arrow
            assert!(svg.contains("<marker")); // Arrow marker
        }

        #[test]
        fn test_annotation_with_label() {
            let screenshot = test_screenshot();
            let annotations = vec![Annotation::rectangle(10, 20, 50, 30).with_label("Test Label")];

            let exporter = SvgExporter::with_config(SvgConfig::new(100, 100));
            let svg = exporter
                .from_screenshot_with_annotations(&screenshot, &annotations)
                .unwrap();

            assert!(svg.contains("<text"));
            assert!(svg.contains("Test Label"));
        }
    }

    mod helper_tests {
        use super::*;

        #[test]
        fn test_escape_xml() {
            assert_eq!(escape_xml("<>&\"'"), "&lt;&gt;&amp;&quot;&apos;");
            assert_eq!(escape_xml("normal text"), "normal text");
            assert_eq!(
                escape_xml("<script>alert('xss')</script>"),
                "&lt;script&gt;alert(&apos;xss&apos;)&lt;/script&gt;"
            );
        }

        #[test]
        fn test_color_to_svg() {
            assert_eq!(color_to_svg(&[255, 0, 0, 255]), "rgba(255,0,0,1)");
            assert_eq!(
                color_to_svg(&[0, 255, 0, 128]),
                "rgba(0,255,0,0.5019607843137255)"
            );
            assert_eq!(color_to_svg(&[0, 0, 0, 0]), "rgba(0,0,0,0)");
        }

        #[test]
        fn test_base64_encode() {
            assert_eq!(base64_encode(b""), "");
            assert_eq!(base64_encode(b"f"), "Zg==");
            assert_eq!(base64_encode(b"fo"), "Zm8=");
            assert_eq!(base64_encode(b"foo"), "Zm9v");
            assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
            assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
            assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        }

        #[test]
        fn test_base64_encode_binary() {
            let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            let encoded = base64_encode(&data);
            assert!(!encoded.is_empty());
            // PNG magic bytes in base64
            assert!(encoded.starts_with("iVBORw"));
        }
    }

    mod property_tests {
        use super::*;

        #[test]
        fn prop_viewbox_matches_config() {
            for width in [100, 800, 1920, 4096] {
                for height in [100, 600, 1080, 2160] {
                    let config = SvgConfig::new(width, height);
                    let exporter = SvgExporter::with_config(config);
                    let screenshot = Screenshot {
                        data: vec![0],
                        width,
                        height,
                        device_pixel_ratio: 1.0,
                        timestamp: SystemTime::now(),
                    };
                    let svg = exporter.from_screenshot(&screenshot).unwrap();

                    assert!(svg.contains(&format!("width=\"{width}\"")));
                    assert!(svg.contains(&format!("height=\"{height}\"")));
                    assert!(svg.contains(&format!("viewBox=\"0 0 {width} {height}\"")));
                }
            }
        }

        #[test]
        fn prop_svg_always_valid_xml() {
            let screenshot = test_screenshot();
            let exporter = SvgExporter::new();
            let svg = exporter.from_screenshot(&screenshot).unwrap();

            // Basic XML validity checks
            assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
            assert!(svg.contains("</svg>"));
            assert_eq!(svg.matches("<svg").count(), 1);
            assert_eq!(svg.matches("</svg>").count(), 1);
        }
    }
}
