//! Coverage Report Formatters (Feature 11, 12, 13)
//!
//! LCOV, HTML, and Cobertura XML format generators for CI integration.
//!
//! ## EXTREME TDD: Tests written FIRST per spec

mod lcov;
mod html;
mod cobertura;

pub use lcov::LcovFormatter;
pub use html::{HtmlFormatter, HtmlReportConfig, Theme};
pub use cobertura::CoberturaFormatter;
