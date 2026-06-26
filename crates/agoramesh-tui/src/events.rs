//! Crossterm key-event mapping to TUI actions.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::Action;
use crate::models::Screen;

/// Translates a crossterm event into an optional TUI action.
///
/// Only key-press events are mapped; repeat and release events are ignored.
#[must_use]
pub fn map_event(event: &Event, screen: Screen) -> Option<Action> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    map_key(*key, screen)
}

fn map_key(key: KeyEvent, screen: Screen) -> Option<Action> {
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::Quit);
    }

    match screen {
        Screen::Compose => match key.code {
            KeyCode::Backspace => Some(Action::ComposeBackspace),
            KeyCode::Tab => Some(Action::ComposeTogglePreview),
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Enter => Some(Action::ComposeSubmit),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveComposeCategory(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveComposeCategory(1)),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::ComposeAppend(ch))
            }
            _ => None,
        },
        Screen::Feed => match key.code {
            KeyCode::Char('1') => Some(Action::SetScreen(Screen::Feed)),
            KeyCode::Char('2') => Some(Action::SetScreen(Screen::Subscriptions)),
            KeyCode::Char('3') => Some(Action::SetScreen(Screen::SyncStatus)),
            KeyCode::Char('4') => Some(Action::SetScreen(Screen::KeyManagement)),
            KeyCode::Char('n') => Some(Action::SetScreen(Screen::Compose)),
            KeyCode::Char('t') | KeyCode::Enter => Some(Action::Select),
            KeyCode::Char('s') => Some(Action::ToggleSelectedSubscription),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSelection(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSelection(1)),
            _ => None,
        },
        Screen::Thread => match key.code {
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSelection(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSelection(1)),
            KeyCode::Char(' ') => Some(Action::ToggleCollapse),
            KeyCode::Enter => Some(Action::Select),
            _ => None,
        },
        Screen::KeyManagement => match key.code {
            KeyCode::Char('g') => Some(Action::GenerateDevKey),
            KeyCode::Esc | KeyCode::Backspace => Some(Action::Back),
            _ => None,
        },
        Screen::Subscriptions => match key.code {
            KeyCode::Char('s') | KeyCode::Enter => Some(Action::ToggleSelectedSubscription),
            KeyCode::Esc | KeyCode::Backspace => Some(Action::Back),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSelection(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSelection(1)),
            _ => None,
        },
        Screen::SyncStatus => match key.code {
            KeyCode::Esc | KeyCode::Backspace => Some(Action::Back),
            KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSelection(-1)),
            KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSelection(1)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_events_map_to_actions() {
        assert_eq!(
            map_event(&press(KeyCode::Char('1')), Screen::Feed),
            Some(Action::SetScreen(crate::models::Screen::Feed))
        );
        assert_eq!(
            map_event(&ctrl(KeyCode::Char('q')), Screen::Feed),
            Some(Action::Quit)
        );
        assert_eq!(
            map_event(&press(KeyCode::Up), Screen::Feed),
            Some(Action::MoveSelection(-1))
        );
        assert_eq!(
            map_event(&press(KeyCode::Char('t')), Screen::Feed),
            Some(Action::Select)
        );
    }

    #[test]
    fn compose_keys_map_to_editor_actions() {
        assert_eq!(
            map_event(&press(KeyCode::Char('a')), Screen::Compose),
            Some(Action::ComposeAppend('a'))
        );
        assert_eq!(
            map_event(&press(KeyCode::Backspace), Screen::Compose),
            Some(Action::ComposeBackspace)
        );
        assert_eq!(
            map_event(&press(KeyCode::Tab), Screen::Compose),
            Some(Action::ComposeTogglePreview)
        );
        assert_eq!(
            map_event(&press(KeyCode::Enter), Screen::Compose),
            Some(Action::ComposeSubmit)
        );
        assert_eq!(
            map_event(&press(KeyCode::Up), Screen::Compose),
            Some(Action::MoveComposeCategory(-1))
        );
    }

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    fn ctrl(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
    }
}
