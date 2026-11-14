//! This module contains the components implemented in the Dappr framework.

/// The button component
pub mod button;
/// Line, bar and scatter charts
pub mod chart;
/// Raw SVG and canvas drawings
pub mod drawing;
/// Typed, labelled inputs (number, slider, checkbox, select, ...)
pub mod field;
/// The html component
pub mod html;
/// The input component
pub mod input;
/// Markdown text and LaTeX equations
#[cfg(any(feature = "markdown", feature = "latex"))]
pub mod markdown;
/// A headline figure
pub mod metric;
/// A status line for data being loaded
pub mod status;
/// A view of a `DTable`
pub mod table;
/// The text component
pub mod text;
