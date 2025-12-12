# Changelog

All notable changes to Probar will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-12-12

### Added

#### Core Testing Framework
- **Locator System**: CSS, XPath, Text, TestId, Entity, Role, Label, Placeholder, AltText selectors
- **Page Object Model**: First-class support for encapsulating UI interactions
- **Fixture System**: Setup/teardown lifecycle with priority ordering
- **Wait Mechanisms**: Custom wait conditions with configurable timeouts

#### Accessibility Testing (WCAG 2.1 AA)
- Color contrast ratio validation (4.5:1 for normal text, 3.0:1 for large text)
- Focus indicator visibility checks
- Semantic structure validation
- Screen reader compatibility helpers

#### Visual Regression Testing
- Golden image comparison with configurable thresholds
- Mask regions for dynamic content exclusion
- Anti-aliasing tolerance options
- Perceptual hash (pHash) for robust frame comparison

#### Device Emulation
- Viewport configuration (mobile, tablet, desktop, ultrawide)
- Device scale factor support
- Touch mode emulation (None, Single, Multi)
- Device descriptor presets (iPhone SE, iPad Mini, Desktop 1080p/4K)

#### UX Coverage Tracking
- Element interaction tracking
- Coverage report generation
- User flow validation

#### Deterministic Replay
- Input recording with frame timestamps
- Reproducible test execution
- Delta encoding for efficient storage

#### Platform Support
- **TUI Testing**: Full ratatui/crossterm integration (default feature)
- **WASM Testing**: `wasm32-unknown-unknown` target support
- **Browser Testing**: Optional CDP/Chromium integration

#### Media Generation
- GIF recording for test artifacts
- PNG snapshot capture
- MP4 video recording (pure Rust)

#### HAR (HTTP Archive) Support
- Network request/response recording
- 50 comprehensive HAR tests

#### Showcase Calculator
- Complete example application demonstrating all Probar features
- 100 H₀ (null hypothesis) tests with EXTREME TDD methodology
- Page Object, Accessibility, Visual Regression, Device Emulation examples

### Documentation
- Comprehensive mdbook documentation
- 100-point QA verification checklist
- Advanced features specification
- Runnable examples (locator_demo, accessibility_demo, coverage_demo, pong_simulation)

### Technical Details
- **Total Tests**: 3,346 passing
- **Test Coverage**: Comprehensive across all modules
- **Rust Version**: 1.75.0+
- **License**: MIT OR Apache-2.0

[0.1.0]: https://github.com/paiml/probar/releases/tag/v0.1.0
