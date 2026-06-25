//! Pubsub topic helpers for object propagation.

/// Returns the object propagation topic for a category scope.
#[must_use]
pub fn topic_for_category(category_id: &str) -> String {
    format!("agoramesh/v0/{category_id}/objects")
}
