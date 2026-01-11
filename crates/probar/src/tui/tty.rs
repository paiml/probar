//! TTY mocking for terminal testing.
//!
//! This module provides mock TTY functionality that captures ANSI escape sequences
//! and tracks terminal state for testing purposes.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::Event;

/// Mock TTY backend that captures output and tracks terminal state.
///
/// This struct allows testing terminal applications without a real TTY by:
/// - Capturing all output (including ANSI escape sequences)
/// - Tracking terminal state (raw mode, alternate screen, etc.)
/// - Providing mock events for input simulation
#[derive(Debug)]
pub struct MockTty {
    output: Vec<u8>,
    size: (u16, u16),
    raw_mode: bool,
    alternate_screen: bool,
    cursor_visible: bool,
    mouse_captured: bool,
    events: VecDeque<Event>,
    poll_results: VecDeque<bool>,
}

impl MockTty {
    /// Create a new mock TTY with the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            output: Vec::new(),
            size: (width, height),
            raw_mode: false,
            alternate_screen: false,
            cursor_visible: true,
            mouse_captured: false,
            events: VecDeque::new(),
            poll_results: VecDeque::new(),
        }
    }

    /// Queue events to be returned by `read_event()`.
    pub fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events.into_iter().collect();
        self
    }

    /// Queue poll results to be returned by `poll()`.
    pub fn with_polls(mut self, polls: Vec<bool>) -> Self {
        self.poll_results = polls.into_iter().collect();
        self
    }

    /// Get the terminal size.
    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    /// Set the terminal size (for resize simulation).
    pub fn set_size(&mut self, width: u16, height: u16) {
        self.size = (width, height);
    }

    /// Check if raw mode is enabled.
    pub fn is_raw_mode(&self) -> bool {
        self.raw_mode
    }

    /// Enable raw mode.
    pub fn enable_raw_mode(&mut self) {
        self.raw_mode = true;
    }

    /// Disable raw mode.
    pub fn disable_raw_mode(&mut self) {
        self.raw_mode = false;
    }

    /// Check if alternate screen is active.
    pub fn is_alternate_screen(&self) -> bool {
        self.alternate_screen
    }

    /// Enter alternate screen.
    pub fn enter_alternate_screen(&mut self) {
        self.alternate_screen = true;
        // Write the escape sequence
        let _ = self.output.write_all(b"\x1b[?1049h");
    }

    /// Leave alternate screen.
    pub fn leave_alternate_screen(&mut self) {
        self.alternate_screen = false;
        let _ = self.output.write_all(b"\x1b[?1049l");
    }

    /// Check if cursor is visible.
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Hide cursor.
    pub fn hide_cursor(&mut self) {
        self.cursor_visible = false;
        let _ = self.output.write_all(b"\x1b[?25l");
    }

    /// Show cursor.
    pub fn show_cursor(&mut self) {
        self.cursor_visible = true;
        let _ = self.output.write_all(b"\x1b[?25h");
    }

    /// Check if mouse capture is enabled.
    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    /// Enable mouse capture.
    pub fn enable_mouse_capture(&mut self) {
        self.mouse_captured = true;
        let _ = self
            .output
            .write_all(b"\x1b[?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h");
    }

    /// Disable mouse capture.
    pub fn disable_mouse_capture(&mut self) {
        self.mouse_captured = false;
        let _ = self
            .output
            .write_all(b"\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l");
    }

    /// Poll for events with timeout.
    pub fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
        Ok(self.poll_results.pop_front().unwrap_or(false))
    }

    /// Read the next event.
    pub fn read_event(&mut self) -> io::Result<Event> {
        self.events
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "no events available"))
    }

    /// Get the captured output bytes.
    pub fn output(&self) -> &[u8] {
        &self.output
    }

    /// Get the captured output as a string (lossy UTF-8 conversion).
    pub fn output_str(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }

    /// Clear the captured output.
    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    /// Check if the output contains a specific byte sequence.
    pub fn output_contains(&self, needle: &[u8]) -> bool {
        self.output
            .windows(needle.len())
            .any(|window| window == needle)
    }

    /// Check if the output contains a specific string.
    pub fn output_contains_str(&self, needle: &str) -> bool {
        self.output_contains(needle.as_bytes())
    }

    /// Check if the output contains an ANSI escape sequence.
    pub fn contains_escape(&self, seq: &str) -> bool {
        let escape_seq = format!("\x1b[{}", seq);
        self.output_contains_str(&escape_seq)
    }

    /// Parse output into ANSI commands.
    pub fn parsed_commands(&self) -> Vec<AnsiCommand> {
        parse_ansi_commands(&self.output)
    }

    /// Get the number of queued events.
    pub fn queued_events(&self) -> usize {
        self.events.len()
    }

    /// Add an event to the queue.
    pub fn push_event(&mut self, event: Event) {
        self.events.push_back(event);
    }

    /// Add a poll result to the queue.
    pub fn push_poll(&mut self, result: bool) {
        self.poll_results.push_back(result);
    }
}

impl Write for MockTty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Default for MockTty {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

/// Parsed ANSI command for testing assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnsiCommand {
    /// Cursor movement: CUP (H), CUU (A), CUD (B), CUF (C), CUB (D)
    CursorMove {
        /// Row position (1-based)
        row: u16,
        /// Column position (1-based)
        col: u16,
    },
    /// Clear screen (ED)
    ClearScreen(ClearMode),
    /// Clear line (EL)
    ClearLine(ClearMode),
    /// Set graphics rendition (SGR)
    SetAttribute(Vec<u8>),
    /// Enter alternate screen
    EnterAlternateScreen,
    /// Leave alternate screen
    LeaveAlternateScreen,
    /// Hide cursor
    HideCursor,
    /// Show cursor
    ShowCursor,
    /// Enable mouse capture
    EnableMouse,
    /// Disable mouse capture
    DisableMouse,
    /// Plain text (non-escape content)
    Text(String),
    /// Unknown or unparsed escape sequence
    Unknown(Vec<u8>),
}

/// Clear mode for ED (erase display) and EL (erase line) commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearMode {
    /// Clear from cursor to end
    ToEnd,
    /// Clear from beginning to cursor
    ToBeginning,
    /// Clear entire screen/line
    All,
}

/// Parse ANSI escape sequences from raw output.
fn parse_ansi_commands(output: &[u8]) -> Vec<AnsiCommand> {
    let mut commands = Vec::new();
    let mut i = 0;
    let mut text_start = 0;

    while i < output.len() {
        if output[i] == 0x1b && i + 1 < output.len() && output[i + 1] == b'[' {
            // Flush pending text
            if text_start < i {
                if let Ok(text) = std::str::from_utf8(&output[text_start..i]) {
                    if !text.is_empty() {
                        commands.push(AnsiCommand::Text(text.to_string()));
                    }
                }
            }

            // Parse CSI sequence
            let seq_start = i;
            i += 2; // Skip ESC [

            // Collect parameter bytes (0x30-0x3F)
            let params_start = i;
            while i < output.len() && (0x30..=0x3F).contains(&output[i]) {
                i += 1;
            }
            let params = &output[params_start..i];

            // Collect intermediate bytes (0x20-0x2F)
            while i < output.len() && (0x20..=0x2F).contains(&output[i]) {
                i += 1;
            }

            // Get final byte (0x40-0x7E)
            if i < output.len() && (0x40..=0x7E).contains(&output[i]) {
                let final_byte = output[i];
                i += 1;

                let cmd = parse_csi_command(params, final_byte);
                commands.push(cmd);
            } else {
                // Incomplete or invalid sequence
                commands.push(AnsiCommand::Unknown(output[seq_start..i].to_vec()));
            }

            text_start = i;
        } else {
            i += 1;
        }
    }

    // Flush remaining text
    if text_start < output.len() {
        if let Ok(text) = std::str::from_utf8(&output[text_start..]) {
            if !text.is_empty() {
                commands.push(AnsiCommand::Text(text.to_string()));
            }
        }
    }

    commands
}

/// Parse a CSI sequence into an AnsiCommand.
fn parse_csi_command(params: &[u8], final_byte: u8) -> AnsiCommand {
    let params_str = std::str::from_utf8(params).unwrap_or("");

    match final_byte {
        b'H' | b'f' => {
            // CUP - Cursor Position
            let parts: Vec<u16> = params_str
                .split(';')
                .filter_map(|s| s.parse().ok())
                .collect();
            let row = parts.first().copied().unwrap_or(1);
            let col = parts.get(1).copied().unwrap_or(1);
            AnsiCommand::CursorMove { row, col }
        }
        b'J' => {
            // ED - Erase Display
            let mode = match params_str {
                "" | "0" => ClearMode::ToEnd,
                "1" => ClearMode::ToBeginning,
                "2" | "3" => ClearMode::All,
                _ => ClearMode::ToEnd,
            };
            AnsiCommand::ClearScreen(mode)
        }
        b'K' => {
            // EL - Erase Line
            let mode = match params_str {
                "" | "0" => ClearMode::ToEnd,
                "1" => ClearMode::ToBeginning,
                "2" => ClearMode::All,
                _ => ClearMode::ToEnd,
            };
            AnsiCommand::ClearLine(mode)
        }
        b'm' => {
            // SGR - Set Graphics Rendition
            let attrs: Vec<u8> = params_str
                .split(';')
                .filter_map(|s| s.parse().ok())
                .collect();
            AnsiCommand::SetAttribute(attrs)
        }
        b'h' => {
            // SM - Set Mode (private modes with ?)
            if params_str == "?1049" {
                AnsiCommand::EnterAlternateScreen
            } else if params_str == "?25" {
                AnsiCommand::ShowCursor
            } else if params_str.starts_with("?1000") || params_str.starts_with("?1002") {
                AnsiCommand::EnableMouse
            } else {
                AnsiCommand::Unknown(format!("\x1b[{}h", params_str).into_bytes())
            }
        }
        b'l' => {
            // RM - Reset Mode (private modes with ?)
            if params_str == "?1049" {
                AnsiCommand::LeaveAlternateScreen
            } else if params_str == "?25" {
                AnsiCommand::HideCursor
            } else if params_str.starts_with("?1000") || params_str.starts_with("?1006") {
                AnsiCommand::DisableMouse
            } else {
                AnsiCommand::Unknown(format!("\x1b[{}l", params_str).into_bytes())
            }
        }
        _ => {
            // Unknown command
            AnsiCommand::Unknown(format!("\x1b[{}{}", params_str, final_byte as char).into_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_new() {
        let tty = MockTty::new(120, 40);
        assert_eq!(tty.size(), (120, 40));
        assert!(!tty.is_raw_mode());
        assert!(!tty.is_alternate_screen());
        assert!(tty.is_cursor_visible());
        assert!(!tty.is_mouse_captured());
    }

    #[test]
    fn test_default() {
        let tty = MockTty::default();
        assert_eq!(tty.size(), (80, 24));
    }

    #[test]
    fn test_raw_mode() {
        let mut tty = MockTty::new(80, 24);
        assert!(!tty.is_raw_mode());
        tty.enable_raw_mode();
        assert!(tty.is_raw_mode());
        tty.disable_raw_mode();
        assert!(!tty.is_raw_mode());
    }

    #[test]
    fn test_alternate_screen() {
        let mut tty = MockTty::new(80, 24);
        assert!(!tty.is_alternate_screen());
        tty.enter_alternate_screen();
        assert!(tty.is_alternate_screen());
        assert!(tty.output_contains_str("\x1b[?1049h"));
        tty.leave_alternate_screen();
        assert!(!tty.is_alternate_screen());
        assert!(tty.output_contains_str("\x1b[?1049l"));
    }

    #[test]
    fn test_cursor_visibility() {
        let mut tty = MockTty::new(80, 24);
        assert!(tty.is_cursor_visible());
        tty.hide_cursor();
        assert!(!tty.is_cursor_visible());
        assert!(tty.output_contains_str("\x1b[?25l"));
        tty.show_cursor();
        assert!(tty.is_cursor_visible());
        assert!(tty.output_contains_str("\x1b[?25h"));
    }

    #[test]
    fn test_mouse_capture() {
        let mut tty = MockTty::new(80, 24);
        assert!(!tty.is_mouse_captured());
        tty.enable_mouse_capture();
        assert!(tty.is_mouse_captured());
        tty.disable_mouse_capture();
        assert!(!tty.is_mouse_captured());
    }

    #[test]
    fn test_write() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"Hello, World!").unwrap();
        assert_eq!(tty.output(), b"Hello, World!");
        assert_eq!(tty.output_str(), "Hello, World!");
    }

    #[test]
    fn test_output_contains() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"Hello, World!").unwrap();
        assert!(tty.output_contains(b"World"));
        assert!(tty.output_contains_str("Hello"));
        assert!(!tty.output_contains_str("Goodbye"));
    }

    #[test]
    fn test_clear_output() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"Hello").unwrap();
        assert!(!tty.output().is_empty());
        tty.clear_output();
        assert!(tty.output().is_empty());
    }

    #[test]
    fn test_events() {
        let tty = MockTty::new(80, 24).with_events(vec![
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)),
        ]);
        assert_eq!(tty.queued_events(), 2);
    }

    #[test]
    fn test_read_event() {
        let mut tty = MockTty::new(80, 24).with_events(vec![Event::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        ))]);
        let event = tty.read_event().unwrap();
        assert!(matches!(event, Event::Key(_)));
        assert!(tty.read_event().is_err()); // No more events
    }

    #[test]
    fn test_poll() {
        let mut tty = MockTty::new(80, 24).with_polls(vec![true, false, true]);
        assert!(tty.poll(Duration::from_millis(100)).unwrap());
        assert!(!tty.poll(Duration::from_millis(100)).unwrap());
        assert!(tty.poll(Duration::from_millis(100)).unwrap());
        assert!(!tty.poll(Duration::from_millis(100)).unwrap()); // Default false
    }

    #[test]
    fn test_push_event() {
        let mut tty = MockTty::new(80, 24);
        assert_eq!(tty.queued_events(), 0);
        tty.push_event(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(tty.queued_events(), 1);
    }

    #[test]
    fn test_push_poll() {
        let mut tty = MockTty::new(80, 24);
        tty.push_poll(true);
        assert!(tty.poll(Duration::ZERO).unwrap());
    }

    #[test]
    fn test_set_size() {
        let mut tty = MockTty::new(80, 24);
        tty.set_size(120, 40);
        assert_eq!(tty.size(), (120, 40));
    }

    #[test]
    fn test_contains_escape() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"\x1b[2J").unwrap(); // Clear screen
        assert!(tty.contains_escape("2J"));
        assert!(!tty.contains_escape("0J"));
    }

    #[test]
    fn test_parsed_commands_cursor_move() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"\x1b[10;20H").unwrap();
        let commands = tty.parsed_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], AnsiCommand::CursorMove { row: 10, col: 20 });
    }

    #[test]
    fn test_parsed_commands_clear_screen() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"\x1b[2J").unwrap();
        let commands = tty.parsed_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], AnsiCommand::ClearScreen(ClearMode::All));
    }

    #[test]
    fn test_parsed_commands_sgr() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"\x1b[1;31m").unwrap(); // Bold red
        let commands = tty.parsed_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], AnsiCommand::SetAttribute(vec![1, 31]));
    }

    #[test]
    fn test_parsed_commands_text() {
        let mut tty = MockTty::new(80, 24);
        tty.write_all(b"Hello\x1b[2JWorld").unwrap();
        let commands = tty.parsed_commands();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], AnsiCommand::Text("Hello".to_string()));
        assert_eq!(commands[1], AnsiCommand::ClearScreen(ClearMode::All));
        assert_eq!(commands[2], AnsiCommand::Text("World".to_string()));
    }

    #[test]
    fn test_parsed_commands_alternate_screen() {
        let mut tty = MockTty::new(80, 24);
        tty.enter_alternate_screen();
        tty.leave_alternate_screen();
        let commands = tty.parsed_commands();
        assert!(commands.contains(&AnsiCommand::EnterAlternateScreen));
        assert!(commands.contains(&AnsiCommand::LeaveAlternateScreen));
    }

    #[test]
    fn test_parsed_commands_cursor_visibility() {
        let mut tty = MockTty::new(80, 24);
        tty.hide_cursor();
        tty.show_cursor();
        let commands = tty.parsed_commands();
        assert!(commands.contains(&AnsiCommand::HideCursor));
        assert!(commands.contains(&AnsiCommand::ShowCursor));
    }
}
