//! TUI application state and reducer.

use std::collections::HashMap;

use crate::compose::ComposeState;
use crate::models::{
    AcknowledgedFirstSeen, CategorySummary, FeedFocus, FeedPost, FirstSeenWarning, KeyInputState,
    KeyStatus, PeerStatus, Screen, Subscriptions, SyncTotals, ThreadView,
};

#[path = "app_state.rs"]
mod app_state;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;

/// User action dispatched from the event loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    /// Switch to another screen.
    SetScreen(Screen),
    /// Move selection up or down in a list.
    MoveSelection(isize),
    /// Select the highlighted item.
    Select,
    /// Do nothing for intentionally ignored input.
    Noop,
    /// Switch feed movement between categories and posts.
    ToggleFeedFocus,
    /// Go back to the previous screen.
    Back,
    /// Quit the application.
    Quit,
    /// Append a character to the compose text editor.
    ComposeAppend(char),
    /// Remove the last character from the compose text editor.
    ComposeBackspace,
    /// Toggle compose preview mode.
    ComposeTogglePreview,
    /// Submit the composed post.
    ComposeSubmit,
    /// Move the selected compose category up or down.
    MoveComposeCategory(isize),
    /// Toggle the subscription state for the selected category.
    ToggleSelectedSubscription,
    /// Generate a development identity key.
    GenerateDevKey,
    /// Append a character to the key passphrase prompt.
    KeyAppend(char),
    /// Remove a character from the key passphrase prompt.
    KeyBackspace,
    /// Unlock an encrypted identity key with the typed passphrase.
    UnlockKey,
    /// Generate an encrypted identity key with the typed passphrase.
    GenerateEncryptedKey,
    /// Backup the current key file.
    BackupKey,
    /// Restore the key file from the default backup.
    RestoreKey,
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
    /// Acknowledge the currently displayed first-seen warning.
    AcknowledgeCurrentWarning,
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
    /// Index of the selected feed category among subscribed feed categories.
    pub selected_category_index: usize,
    /// Index of the selected post within the selected feed category.
    pub selected_post_index: usize,
    /// Currently focused feed pane.
    pub feed_focus: FeedFocus,
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
    /// Key-management passphrase/action state.
    pub key_input: KeyInputState,
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
            selected_category_index: 0,
            selected_post_index: 0,
            feed_focus: FeedFocus::Categories,
            subscriptions: Subscriptions::default(),
            categories: Vec::new(),
            peers: Vec::new(),
            sync_totals: SyncTotals::default(),
            posts: HashMap::new(),
            key_status: KeyStatus::Missing,
            key_input: KeyInputState::default(),
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
            Action::Noop
            | Action::Select
            | Action::ComposeSubmit
            | Action::UnlockKey
            | Action::GenerateEncryptedKey
            | Action::GenerateDevKey
            | Action::BackupKey
            | Action::RestoreKey
            | Action::AcknowledgeCurrentWarning => {}
            Action::ToggleFeedFocus => {
                self.feed_focus = match self.feed_focus {
                    FeedFocus::Categories => FeedFocus::Posts,
                    FeedFocus::Posts => FeedFocus::Categories,
                };
            }
            Action::MoveSelection(delta) => {
                self.move_selection(delta);
            }
            Action::KeyAppend(ch) => {
                self.key_input.passphrase.push(ch);
            }
            Action::KeyBackspace => {
                self.key_input.passphrase.pop();
            }
            Action::ComposeAppend(ch) => {
                self.compose.text.push(ch);
            }
            Action::ComposeBackspace => {
                self.compose.text.pop();
            }
            Action::ComposeTogglePreview => {
                self.compose.preview = !self.compose.preview;
            }
            Action::MoveComposeCategory(delta) => {
                self.move_compose_category(delta);
            }
            Action::ToggleSelectedSubscription => {
                self.toggle_selected_subscription();
            }
            Action::Back => {
                self.go_back();
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SetSubscriptions(subscriptions) => self.subscriptions = subscriptions,
            Action::SetCategories(categories) => self.set_categories(categories),
            Action::SetPeers(peers) => self.set_peers(peers),
            Action::SetSyncTotals(totals) => self.sync_totals = totals,
            Action::SetPosts(posts) => self.set_posts(posts),
            Action::SetKeyStatus(status) => self.key_status = status,
            Action::SetWarnings(warnings) => self.warnings = warnings,
            Action::AcknowledgeWarning(warning) => {
                self.acknowledge_warning(warning);
            }
            Action::SetAcknowledged(acknowledged) => self.acknowledged = acknowledged,
            Action::SetThread(thread) => {
                self.thread = Some(thread);
            }
            Action::ToggleCollapse => {
                if let Some(thread) = &mut self.thread {
                    app_state::toggle_at_index(&mut thread.comments, self.selected_index);
                }
            }
        }
        self
    }

    fn toggle_selected_subscription(&mut self) {
        let Some(category_id) = self.selected_category_id_for_subscription_toggle() else {
            return;
        };
        if let Some(index) = self
            .subscriptions
            .category_ids
            .iter()
            .position(|item| item == &category_id)
        {
            self.subscriptions.category_ids.remove(index);
        } else {
            self.subscriptions.category_ids.push(category_id);
        }
    }

    fn go_back(&mut self) {
        if let Some(screen) = self.screen_stack.pop() {
            self.screen = screen;
            self.selected_index = 0;
        }
    }

    fn set_categories(&mut self, categories: Vec<CategorySummary>) {
        let new_index = self.selected_index.min(categories.len().saturating_sub(1));
        self.categories = categories;
        self.selected_index = new_index;
        self.clamp_feed_post_index();
    }

    fn set_peers(&mut self, peers: Vec<PeerStatus>) {
        let new_index = self.selected_index.min(peers.len().saturating_sub(1));
        self.peers = peers;
        self.selected_index = new_index;
    }

    fn set_posts(&mut self, posts: HashMap<String, Vec<FeedPost>>) {
        self.posts = posts;
        self.clamp_feed_post_index();
    }

    fn acknowledge_warning(&mut self, warning: FirstSeenWarning) {
        self.warnings.retain(|item| item != &warning);
        match warning {
            FirstSeenWarning::Category { category_id, .. } => {
                push_unique(&mut self.acknowledged.categories, category_id);
            }
            FirstSeenWarning::Peer { address } => {
                push_unique(&mut self.acknowledged.peers, address);
            }
        }
    }
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}
