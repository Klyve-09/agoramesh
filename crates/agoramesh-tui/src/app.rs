//! TUI application state and reducer.

use std::collections::HashMap;

use crate::compose::ComposeState;
use crate::models::{
    AcknowledgedFirstSeen, CategorySummary, FeedPost, FirstSeenWarning, KeyStatus, PeerStatus,
    Screen, Subscriptions, SyncTotals, ThreadView,
};

/// User action dispatched from the event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Switch to another screen.
    SetScreen(Screen),
    /// Move selection up or down in a list.
    MoveSelection(isize),
    /// Select the highlighted item.
    Select,
    /// Go back to the previous screen.
    Back,
    /// Quit the application.
    Quit,
    /// Append a character to the compose text editor.
    ComposeAppend(char),
    /// Toggle compose preview mode.
    ComposeTogglePreview,
    /// Submit the composed post.
    ComposeSubmit,
    /// Update the locally subscribed category list.
    SetSubscriptions(Subscriptions),
    /// Update the displayed categories.
    SetCategories(Vec<CategorySummary>),
    /// Update the displayed peer statuses.
    SetPeers(Vec<PeerStatus>),
    /// Update the cached posts map.
    SetPosts(HashMap<String, Vec<FeedPost>>),
    /// Update the latest sync totals.
    SetSyncTotals(SyncTotals),
    /// Update key status.
    SetKeyStatus(KeyStatus),
    /// Update the active first-seen warnings.
    SetWarnings(Vec<FirstSeenWarning>),
    /// Acknowledge a warning, removing it from the active set.
    AcknowledgeWarning(FirstSeenWarning),
    /// Update the acknowledged-first-seen persistence set.
    SetAcknowledged(AcknowledgedFirstSeen),
    /// Update the thread view for a selected post.
    SetThread(ThreadView),
    /// Toggle collapse on the selected thread comment.
    ToggleCollapse,
}

/// Central application state for the TUI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppState {
    /// Currently visible screen.
    pub screen: Screen,
    /// Stack of previous screens for Back navigation.
    pub screen_stack: Vec<Screen>,
    /// Index of the selected list item on the current screen.
    pub selected_index: usize,
    /// Locally subscribed category IDs.
    pub subscriptions: Subscriptions,
    /// Known categories loaded from the store.
    pub categories: Vec<CategorySummary>,
    /// Peer statuses displayed in the sync panel.
    pub peers: Vec<PeerStatus>,
    /// Latest sync result totals.
    pub sync_totals: SyncTotals,
    /// Cached posts keyed by category ID.
    pub posts: HashMap<String, Vec<FeedPost>>,
    /// Key management panel state.
    pub key_status: KeyStatus,
    /// Active first-seen warnings.
    pub warnings: Vec<FirstSeenWarning>,
    /// Acknowledged first-seen values.
    pub acknowledged: AcknowledgedFirstSeen,
    /// Mutable compose state for the post editor.
    pub compose: ComposeState,
    /// Currently viewed thread, if any.
    pub thread: Option<ThreadView>,
    /// Whether the application should exit.
    pub should_quit: bool,
    /// Optional status message shown in the status bar.
    pub status_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::Feed,
            screen_stack: Vec::new(),
            selected_index: 0,
            subscriptions: Subscriptions::default(),
            categories: Vec::new(),
            peers: Vec::new(),
            sync_totals: SyncTotals::default(),
            posts: HashMap::new(),
            key_status: KeyStatus::Missing,
            warnings: Vec::new(),
            acknowledged: AcknowledgedFirstSeen::default(),
            compose: ComposeState::default(),
            thread: None,
            should_quit: false,
            status_message: None,
        }
    }
}

impl AppState {
    /// Create a new default application state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an action to the state and return the updated state.
    #[must_use]
    pub fn apply(mut self, action: Action) -> Self {
        match action {
            Action::SetScreen(screen) => {
                self.screen_stack.push(self.screen);
                self.screen = screen;
                self.selected_index = 0;
            }
            Action::MoveSelection(delta) => {
                self.move_selection(delta);
            }
            Action::Select | Action::ComposeSubmit => {
                // Selection/submission handling is screen-specific and performed
                // by the caller after the reducer returns the updated state.
            }
            Action::ComposeAppend(ch) => {
                self.compose.text.push(ch);
            }
            Action::ComposeTogglePreview => {
                self.compose.preview = !self.compose.preview;
            }
            Action::Back => {
                if let Some(screen) = self.screen_stack.pop() {
                    self.screen = screen;
                    self.selected_index = 0;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SetSubscriptions(subscriptions) => {
                self.subscriptions = subscriptions;
            }
            Action::SetCategories(categories) => {
                let new_index = self.selected_index.min(categories.len().saturating_sub(1));
                self.categories = categories;
                self.selected_index = new_index;
            }
            Action::SetPeers(peers) => {
                let new_index = self.selected_index.min(peers.len().saturating_sub(1));
                self.peers = peers;
                self.selected_index = new_index;
            }
            Action::SetSyncTotals(totals) => {
                self.sync_totals = totals;
            }
            Action::SetPosts(posts) => {
                self.posts = posts;
            }
            Action::SetKeyStatus(status) => {
                self.key_status = status;
            }
            Action::SetWarnings(warnings) => {
                self.warnings = warnings;
            }
            Action::AcknowledgeWarning(warning) => {
                self.warnings.retain(|item| item != &warning);
                match warning {
                    FirstSeenWarning::Category { category_id, .. } => {
                        if !self.acknowledged.categories.contains(&category_id) {
                            self.acknowledged.categories.push(category_id);
                        }
                    }
                    FirstSeenWarning::Peer { address } => {
                        if !self.acknowledged.peers.contains(&address) {
                            self.acknowledged.peers.push(address);
                        }
                    }
                }
            }
            Action::SetAcknowledged(acknowledged) => {
                self.acknowledged = acknowledged;
            }
            Action::SetThread(thread) => {
                self.thread = Some(thread);
            }
            Action::ToggleCollapse => {
                if let Some(thread) = &mut self.thread {
                    toggle_at_index(&mut thread.comments, self.selected_index);
                }
            }
        }
        self
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.list_len();
        if len == 0 {
            self.selected_index = 0;
            return;
        }
        let current = isize::try_from(self.selected_index).unwrap_or(0);
        let len_isize = isize::try_from(len).unwrap_or(1);
        let next = current.wrapping_add(delta).rem_euclid(len_isize);
        self.selected_index = usize::try_from(next).unwrap_or(0);
    }

    fn list_len(&self) -> usize {
        match self.screen {
            Screen::Feed => self.categories.len(),
            Screen::Subscriptions => self.subscriptions.category_ids.len(),
            Screen::SyncStatus => self.peers.len(),
            Screen::Thread => self
                .thread
                .as_ref()
                .map_or(0, |thread| count_comments(&thread.comments)),
            _ => 0,
        }
    }
}

fn count_comments(comments: &[crate::models::ThreadComment]) -> usize {
    comments
        .iter()
        .map(|comment| count_comments(&comment.replies).saturating_add(1))
        .sum::<usize>()
}

fn toggle_at_index(comments: &mut [crate::models::ThreadComment], mut index: usize) {
    for comment in comments {
        if index == 0 {
            comment.collapsed = !comment.collapsed;
            return;
        }
        index = index.saturating_sub(1);
        let reply_count = count_comments(&comment.replies);
        if index < reply_count {
            toggle_at_index(&mut comment.replies, index);
            return;
        }
        index = index.saturating_sub(reply_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_switches_screens_when_actions_are_applied() {
        let state = AppState::new();
        assert_eq!(state.screen, Screen::Feed);

        let state = state.apply(Action::SetScreen(Screen::Subscriptions));
        assert_eq!(state.screen, Screen::Subscriptions);
        assert_eq!(state.screen_stack, vec![Screen::Feed]);

        let state = state.apply(Action::Back);
        assert_eq!(state.screen, Screen::Feed);
        assert!(state.screen_stack.is_empty());
    }

    #[test]
    fn quit_action_sets_should_quit() {
        let state = AppState::new().apply(Action::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn acknowledging_category_warning_moves_it_to_acknowledged() {
        let warning = FirstSeenWarning::Category {
            category_id: "cat-1".to_owned(),
            display_name: None,
        };
        let state = AppState::new()
            .apply(Action::SetWarnings(vec![warning.clone()]))
            .apply(Action::AcknowledgeWarning(warning));

        assert!(state.warnings.is_empty());
        assert_eq!(state.acknowledged.categories, vec!["cat-1".to_owned()]);
    }
}
