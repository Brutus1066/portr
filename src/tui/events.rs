//! Event handling for the TUI dashboard
//!
//! Keyboard and mouse event processing.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Keyboard action that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    MoveToFirst,
    MoveToLast,
    Kill,
    Refresh,
    CycleFilter,
    CycleSort,
    ToggleDetails,
    ToggleHelp,
    ToggleMenu,
    ToggleCritical,
    ToggleDocker,
    StartSearch,
    MenuSelect(usize),
    None,
}

/// Convert a key event to an action
pub fn key_to_action(key: KeyEvent, in_menu: bool) -> Action {
    if in_menu {
        // Menu-specific keybindings
        return match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('m') => Action::ToggleMenu,
            KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
            KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
            KeyCode::Enter => Action::MenuSelect(0), // Placeholder, actual index from app
            KeyCode::Char('1') => Action::MenuSelect(0),
            KeyCode::Char('2') => Action::MenuSelect(1),
            KeyCode::Char('3') => Action::MenuSelect(2),
            KeyCode::Char('4') => Action::MenuSelect(3),
            KeyCode::Char('5') => Action::MenuSelect(4),
            _ => Action::None,
        };
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Esc => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('g') => Action::MoveToFirst,
        KeyCode::Char('G') => Action::MoveToLast,
        KeyCode::Home => Action::MoveToFirst,
        KeyCode::End => Action::MoveToLast,

        // Actions
        KeyCode::Char('K') => Action::Kill,
        KeyCode::Char('r') | KeyCode::F(5) => Action::Refresh,

        // Filters and sorting
        KeyCode::Char('f') => Action::CycleFilter,
        KeyCode::Tab => Action::CycleSort,
        KeyCode::Char('c') => Action::ToggleCritical,
        KeyCode::Char('d') => Action::ToggleDocker,
        KeyCode::Char('/') => Action::StartSearch,
        KeyCode::Char('m') => Action::ToggleMenu,

        // Display toggles
        KeyCode::Enter => Action::ToggleDetails,
        KeyCode::Char('?') => Action::ToggleHelp,

        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quit_actions() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(key_to_action(key, false), Action::Quit);
    }

    #[test]
    fn test_navigation() {
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(key_to_action(key_j, false), Action::MoveDown);
        assert_eq!(key_to_action(key_k, false), Action::MoveUp);
    }
}
