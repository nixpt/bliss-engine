//! Style types for the declarative Element builder API.
//!
//! These types define the CSS-like properties that can be set on an
//! [`Element`](crate::Element) before it is mounted to a [`Window`](crate::Window).
//! When mounted, the style is serialised to an inline CSS string and set via
//! the element's `style` attribute.

// ---------------------------------------------------------------------------
// Re-exported types
// ---------------------------------------------------------------------------

/// A CSS dimension value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    /// Percentage of the containing block (value 0.0 – 100.0)
    Percent(f32),
    /// Fixed pixel value
    Px(f32),
    /// Automatic sizing
    Auto,
}

impl Dimension {
    /// Convert to a CSS string fragment (without the property name).
    pub fn to_css(&self) -> String {
        match self {
            Dimension::Percent(v) => format!("{}%", v),
            Dimension::Px(v) => format!("{}px", v),
            Dimension::Auto => "auto".to_string(),
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

/// A rectangle of [`Dimension`] values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub top: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub left: Dimension,
}

impl Rect {
    /// Create a rectangle with uniform padding on all sides.
    pub const fn uniform(v: f32) -> Self {
        let d = Dimension::Px(v);
        Rect {
            top: d,
            right: d,
            bottom: d,
            left: d,
        }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Rect {
            top: Dimension::Auto,
            right: Dimension::Auto,
            bottom: Dimension::Auto,
            left: Dimension::Auto,
        }
    }
}

/// Display behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    /// Block layout.
    Block,
    /// Flexbox layout.
    Flex,
    /// Inline layout.
    Inline,
    /// Inline-block layout.
    InlineBlock,
    /// No display.
    None,
}

impl Display {
    pub fn to_css(&self) -> &str {
        match self {
            Display::Block => "block",
            Display::Flex => "flex",
            Display::Inline => "inline",
            Display::InlineBlock => "inline-block",
            Display::None => "none",
        }
    }
}

/// Flex container direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    /// Left-to-right row.
    Row,
    /// Right-to-left row.
    RowReverse,
    /// Top-to-bottom column.
    Column,
    /// Bottom-to-top column.
    ColumnReverse,
}

impl FlexDirection {
    pub fn to_css(&self) -> &str {
        match self {
            FlexDirection::Row => "row",
            FlexDirection::RowReverse => "row-reverse",
            FlexDirection::Column => "column",
            FlexDirection::ColumnReverse => "column-reverse",
        }
    }
}

/// Cross-axis alignment for flex items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    /// Stretch to fill the container.
    Stretch,
    /// Centered.
    Center,
    /// Start of the cross axis.
    FlexStart,
    /// End of the cross axis.
    FlexEnd,
    /// Baseline alignment.
    Baseline,
}

impl AlignItems {
    pub fn to_css(&self) -> &str {
        match self {
            AlignItems::Stretch => "stretch",
            AlignItems::Center => "center",
            AlignItems::FlexStart => "flex-start",
            AlignItems::FlexEnd => "flex-end",
            AlignItems::Baseline => "baseline",
        }
    }
}

/// Main-axis alignment for flex items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JustifyContent {
    /// Centered.
    Center,
    /// Start of the main axis.
    FlexStart,
    /// End of the main axis.
    FlexEnd,
    /// Space between items.
    SpaceBetween,
    /// Space around items.
    SpaceAround,
    /// Space evenly distributed.
    SpaceEvenly,
}

impl JustifyContent {
    pub fn to_css(&self) -> &str {
        match self {
            JustifyContent::Center => "center",
            JustifyContent::FlexStart => "flex-start",
            JustifyContent::FlexEnd => "flex-end",
            JustifyContent::SpaceBetween => "space-between",
            JustifyContent::SpaceAround => "space-around",
            JustifyContent::SpaceEvenly => "space-evenly",
        }
    }
}

/// Overflow behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overflow {
    /// Automatically add scrollbars when content overflows.
    Auto,
    /// Overflowing content is hidden.
    Hidden,
    /// Overflowing content is visible.
    Visible,
    /// Always show scrollbars.
    Scroll,
}

impl Overflow {
    pub fn to_css(&self) -> &str {
        match self {
            Overflow::Auto => "auto",
            Overflow::Hidden => "hidden",
            Overflow::Visible => "visible",
            Overflow::Scroll => "scroll",
        }
    }
}

/// White-space handling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhiteSpace {
    /// Normal collapsing.
    Normal,
    /// Preserve whitespace and wrap.
    PreWrap,
    /// No wrapping.
    Nowrap,
    /// Preserve whitespace, no wrap.
    Pre,
}

impl WhiteSpace {
    pub fn to_css(&self) -> &str {
        match self {
            WhiteSpace::Normal => "normal",
            WhiteSpace::PreWrap => "pre-wrap",
            WhiteSpace::Nowrap => "nowrap",
            WhiteSpace::Pre => "pre",
        }
    }
}

/// Cursor style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cursor {
    /// Default arrow cursor.
    Default,
    /// Pointer / hand cursor.
    Pointer,
    /// Text caret cursor.
    Text,
}

impl Cursor {
    pub fn to_css(&self) -> &str {
        match self {
            Cursor::Default => "default",
            Cursor::Pointer => "pointer",
            Cursor::Text => "text",
        }
    }
}

/// Font weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    /// Normal weight (400).
    Normal,
    /// Bold weight (700).
    Bold,
    /// Lighter weight (300).
    Light,
}

impl FontWeight {
    pub fn to_css(&self) -> &str {
        match self {
            FontWeight::Normal => "400",
            FontWeight::Bold => "700",
            FontWeight::Light => "300",
        }
    }
}

/// Font style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    /// Normal style.
    Normal,
    /// Italic style.
    Italic,
    /// Oblique style.
    Oblique,
}

impl FontStyle {
    pub fn to_css(&self) -> &str {
        match self {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        }
    }
}

/// Text transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextTransform {
    /// Uppercase transform.
    Uppercase,
    /// Lowercase transform.
    Lowercase,
    /// Capitalize transform.
    Capitalize,
}

impl TextTransform {
    pub fn to_css(&self) -> &str {
        match self {
            TextTransform::Uppercase => "uppercase",
            TextTransform::Lowercase => "lowercase",
            TextTransform::Capitalize => "capitalize",
        }
    }
}

/// A CSS border declaration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// Border width in pixels.
    pub width: f32,
    /// Border colour.
    pub color: crate::Color,
}

impl Default for BorderStyle {
    fn default() -> Self {
        BorderStyle::Solid
    }
}

impl Default for Border {
    fn default() -> Self {
        Border {
            width: 1.0,
            color: crate::Color::rgb(0, 0, 0),
        }
    }
}

impl Border {
    /// Convert to a CSS `border-*` shorthand value.
    pub fn to_css(&self) -> String {
        format!("{}px solid {}", self.width, self.color.to_css())
    }
}

/// Border line style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    /// Solid line.
    Solid,
    /// Dashed line.
    Dashed,
    /// Dotted line.
    Dotted,
    /// No border.
    None,
}

impl BorderStyle {
    pub fn to_css(&self) -> &str {
        match self {
            BorderStyle::Solid => "solid",
            BorderStyle::Dashed => "dashed",
            BorderStyle::Dotted => "dotted",
            BorderStyle::None => "none",
        }
    }
}

// ---------------------------------------------------------------------------
// Style struct — mirrors the fields used across cece-code/src/ui.rs
// ---------------------------------------------------------------------------

/// A set of CSS-like style declarations.
///
/// Every field is optional — `None` means "not specified". When an
/// [`Element`](crate::Element) is mounted, non-`None` fields are serialised
/// into an inline `style` attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub display: Option<Display>,
    pub flex_direction: Option<FlexDirection>,
    pub flex_grow: Option<f32>,
    pub align_items: Option<AlignItems>,
    pub justify_content: Option<JustifyContent>,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub min_width: Option<Dimension>,
    pub min_height: Option<Dimension>,
    pub padding: Option<Rect>,
    pub padding_left: Option<Dimension>,
    pub margin_left: Option<Dimension>,
    pub margin_right: Option<Dimension>,
    pub margin_bottom: Option<Dimension>,
    pub border: Option<Border>,
    pub border_top: Option<Border>,
    pub border_bottom: Option<Border>,
    pub border_left: Option<Border>,
    pub border_right: Option<Border>,
    pub color: Option<crate::Color>,
    pub background_color: Option<crate::Color>,
    pub font_size: Option<Dimension>,
    pub font_family: Option<String>,
    pub line_height: Option<Dimension>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub text_transform: Option<TextTransform>,
    pub overflow: Option<Overflow>,
    pub white_space: Option<WhiteSpace>,
    pub cursor: Option<Cursor>,
    pub gap: Option<Dimension>,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            display: None,
            flex_direction: None,
            flex_grow: None,
            align_items: None,
            justify_content: None,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            padding: None,
            padding_left: None,
            margin_left: None,
            margin_right: None,
            margin_bottom: None,
            border: None,
            border_top: None,
            border_bottom: None,
            border_left: None,
            border_right: None,
            color: None,
            background_color: None,
            font_size: None,
            font_family: None,
            line_height: None,
            font_weight: None,
            font_style: None,
            text_transform: None,
            overflow: None,
            white_space: None,
            cursor: None,
            gap: None,
        }
    }
}

impl Style {
    /// Serialise non-`None` style fields into an inline CSS string.
    pub fn to_inline_css(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(v) = &self.display {
            parts.push(format!("display:{}", v.to_css()));
        }
        if let Some(v) = &self.flex_direction {
            parts.push(format!("flex-direction:{}", v.to_css()));
        }
        if let Some(v) = &self.flex_grow {
            parts.push(format!("flex-grow:{}", v));
        }
        if let Some(v) = &self.align_items {
            parts.push(format!("align-items:{}", v.to_css()));
        }
        if let Some(v) = &self.justify_content {
            parts.push(format!("justify-content:{}", v.to_css()));
        }
        if let Some(v) = &self.width {
            parts.push(format!("width:{}", v.to_css()));
        }
        if let Some(v) = &self.height {
            parts.push(format!("height:{}", v.to_css()));
        }
        if let Some(v) = &self.min_width {
            parts.push(format!("min-width:{}", v.to_css()));
        }
        if let Some(v) = &self.min_height {
            parts.push(format!("min-height:{}", v.to_css()));
        }
        if let Some(v) = &self.padding {
            let t = self.dim_css(&v.top);
            let r = self.dim_css(&v.right);
            let b = self.dim_css(&v.bottom);
            let l = self.dim_css(&v.left);
            parts.push(format!("padding:{} {} {} {}", t, r, b, l));
        }
        if let Some(v) = &self.padding_left {
            parts.push(format!("padding-left:{}", v.to_css()));
        }
        if let Some(v) = &self.margin_left {
            parts.push(format!("margin-left:{}", v.to_css()));
        }
        if let Some(v) = &self.margin_right {
            parts.push(format!("margin-right:{}", v.to_css()));
        }
        if let Some(v) = &self.margin_bottom {
            parts.push(format!("margin-bottom:{}", v.to_css()));
        }
        if let Some(v) = &self.border {
            parts.push(format!("border:{}", v.to_css()));
        }
        if let Some(v) = &self.border_top {
            parts.push(format!("border-top:{}", v.to_css()));
        }
        if let Some(v) = &self.border_bottom {
            parts.push(format!("border-bottom:{}", v.to_css()));
        }
        if let Some(v) = &self.border_left {
            parts.push(format!("border-left:{}", v.to_css()));
        }
        if let Some(v) = &self.border_right {
            parts.push(format!("border-right:{}", v.to_css()));
        }
        if let Some(v) = &self.color {
            parts.push(format!("color:{}", v.to_css()));
        }
        if let Some(v) = &self.background_color {
            parts.push(format!("background-color:{}", v.to_css()));
        }
        if let Some(v) = &self.font_size {
            parts.push(format!("font-size:{}", v.to_css()));
        }
        if let Some(v) = &self.font_family {
            parts.push(format!("font-family:{}", v));
        }
        if let Some(v) = &self.line_height {
            parts.push(format!("line-height:{}", v.to_css()));
        }
        if let Some(v) = &self.font_weight {
            parts.push(format!("font-weight:{}", v.to_css()));
        }
        if let Some(v) = &self.font_style {
            parts.push(format!("font-style:{}", v.to_css()));
        }
        if let Some(v) = &self.text_transform {
            parts.push(format!("text-transform:{}", v.to_css()));
        }
        if let Some(v) = &self.overflow {
            parts.push(format!("overflow:{}", v.to_css()));
        }
        if let Some(v) = &self.white_space {
            parts.push(format!("white-space:{}", v.to_css()));
        }
        if let Some(v) = &self.cursor {
            parts.push(format!("cursor:{}", v.to_css()));
        }
        if let Some(v) = &self.gap {
            parts.push(format!("gap:{}", v.to_css()));
        }

        parts.join(";")
    }

    fn dim_css(&self, d: &Dimension) -> String {
        match d {
            Dimension::Percent(v) => format!("{}%", v),
            Dimension::Px(v) => format!("{}px", v),
            Dimension::Auto => "auto".to_string(),
        }
    }
}
