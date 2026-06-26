use super::AppState;
use crate::models::Screen;

impl AppState {
    pub(super) fn move_selection(&mut self, delta: isize) {
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

    pub(super) fn move_compose_category(&mut self, delta: isize) {
        let len = self.categories.len();
        if len == 0 {
            self.compose.category_index = 0;
            return;
        }
        let current = isize::try_from(self.compose.category_index).unwrap_or(0);
        let len_isize = isize::try_from(len).unwrap_or(1);
        let next = current.wrapping_add(delta).rem_euclid(len_isize);
        self.compose.category_index = usize::try_from(next).unwrap_or(0);
    }

    pub(super) fn selected_category_id_for_subscription_toggle(&self) -> Option<String> {
        match self.screen {
            Screen::Feed => self
                .categories
                .get(self.selected_index)
                .map(|category| category.category_id.clone()),
            Screen::Subscriptions => self
                .subscriptions
                .category_ids
                .get(self.selected_index)
                .cloned(),
            _ => None,
        }
    }
}

fn count_comments(comments: &[crate::models::ThreadComment]) -> usize {
    comments
        .iter()
        .map(|comment| count_comments(&comment.replies).saturating_add(1))
        .sum::<usize>()
}

pub(super) fn toggle_at_index(comments: &mut [crate::models::ThreadComment], mut index: usize) {
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
