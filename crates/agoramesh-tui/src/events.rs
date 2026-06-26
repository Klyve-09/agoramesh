//! Crossterm key-event mapping to TUI actions.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::Action;

/// Translates a crossterm event into an optional TUI action.
///
/// Only key-press events are mapped; repeat and release events are ignored.
#[must_use]
pub fn map_event(event: &Event) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    Some(map_key(*key))
}

fn map_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => Action::Quit,
        KeyCode::Char('2') => Action::SetScreen(crate::models::Screen::Subscriptions),
        KeyCode::Char('3') => Action::SetScreen(crate::models::Screen::SyncStatus),
        KeyCode::Char('4') => Action::SetScreen(crate::models::Screen::KeyManagement),
        KeyCode::Char('n') => Action::SetScreen(crate::models::Screen::Compose),
        KeyCode::Char('t') => Action::SetScreen(crate::models::Screen::Thread),
        KeyCode::Esc => Action::Back,
        KeyCode::Up | KeyCode::Char('k') => Action::MoveSelection(-1),
        KeyCode::Down | KeyCode::Char('j') => Action::MoveSelection(1),
        KeyCode::Enter => Action::Select,
        KeyCode::Char(' ') => Action::ToggleCollapse,
        _ => Action::SetScreen(crate::models::Screen::Feed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_events_map_to_actions() {
        assert_eq!(
            map_event(&press(KeyCode::Char('1'))),
            Some(Action::SetScreen(crate::models::Screen::Feed))
        );
        assert_eq!(map_event(&ctrl(KeyCode::Char('q'))), Some(Action::Quit));
        assert_eq!(
            map_event(&press(KeyCode::Up)),
            Some(Action::MoveSelection(-1))
        );
    }

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    fn ctrl(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
    }
}
