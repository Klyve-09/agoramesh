use super::AppState;
use crate::models::{CategorySummary, FeedFocus, FeedPost, Screen};

impl AppState {
    pub(super) fn move_selection(&mut self, delta: isize) {
        if self.screen == Screen::Feed {
            self.move_feed_selection(delta);
            return;
        }

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
            Screen::Feed => self.visible_feed_categories().len(),
            Screen::Subscriptions => self.categories.len(),
            Screen::SyncStatus => self.peers.len(),
            Screen::Thread => self
                .thread
                .as_ref()
                .map_or(0, |thread| count_visible_comments(&thread.comments)),
            _ => 0,
        }
    }

    fn move_feed_selection(&mut self, delta: isize) {
        match self.feed_focus {
            FeedFocus::Categories => {
                self.selected_category_index = moved_index(
                    self.selected_category_index,
                    delta,
                    self.visible_feed_categories().len(),
                );
                self.clamp_feed_post_index();
            }
            FeedFocus::Posts => {
                self.selected_post_index = moved_index(
                    self.selected_post_index,
                    delta,
                    self.selected_feed_posts().len(),
                );
            }
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
                .selected_feed_category()
                .map(|category| category.category_id.clone()),
            Screen::Subscriptions => self
                .categories
                .get(self.selected_index)
                .map(|category| category.category_id.clone()),
            _ => None,
        }
    }

    pub(crate) fn visible_feed_categories(&self) -> Vec<&CategorySummary> {
        self.categories
            .iter()
            .filter(|category| {
                self.subscriptions
                    .category_ids
                    .contains(&category.category_id)
            })
            .collect()
    }

    pub(crate) fn selected_feed_category(&self) -> Option<&CategorySummary> {
        self.visible_feed_categories()
            .into_iter()
            .nth(self.selected_category_index)
    }

    pub(crate) fn selected_feed_posts(&self) -> &[FeedPost] {
        self.selected_feed_category()
            .and_then(|category| self.posts.get(&category.category_id))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn selected_feed_post(&self) -> Option<&FeedPost> {
        self.selected_feed_posts().get(self.selected_post_index)
    }

    pub(crate) fn clamp_feed_post_index(&mut self) {
        let categories_len = self.visible_feed_categories().len();
        if categories_len == 0 {
            self.selected_category_index = 0;
            self.selected_post_index = 0;
            return;
        }
        self.selected_category_index = self
            .selected_category_index
            .min(categories_len.saturating_sub(1));
        let posts_len = self.selected_feed_posts().len();
        if posts_len == 0 {
            self.selected_post_index = 0;
        } else {
            self.selected_post_index = self.selected_post_index.min(posts_len.saturating_sub(1));
        }
    }
}

fn moved_index(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let current = isize::try_from(current).unwrap_or(0);
    let len_isize = isize::try_from(len).unwrap_or(1);
    let next = current.wrapping_add(delta).rem_euclid(len_isize);
    usize::try_from(next).unwrap_or(0)
}

fn count_visible_comments(comments: &[crate::models::ThreadComment]) -> usize {
    comments
        .iter()
        .map(|comment| {
            if comment.collapsed {
                1
            } else {
                count_visible_comments(&comment.replies).saturating_add(1)
            }
        })
        .sum::<usize>()
}

pub(super) fn toggle_at_index(comments: &mut [crate::models::ThreadComment], mut index: usize) {
    for comment in comments {
        if index == 0 {
            comment.collapsed = !comment.collapsed;
            return;
        }
        index = index.saturating_sub(1);
        let reply_count = if comment.collapsed {
            0
        } else {
            count_visible_comments(&comment.replies)
        };
        if reply_count > 0 && index < reply_count {
            toggle_at_index(&mut comment.replies, index);
            return;
        }
        index = index.saturating_sub(reply_count);
    }
}
