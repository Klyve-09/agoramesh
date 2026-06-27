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

#[test]
fn compose_actions_update_local_editor_state() {
    let mut state = AppState::new();
    state.categories = vec![
        CategorySummary {
            object_id: "oid-1".to_owned(),
            display_name: "General".to_owned(),
            description: String::new(),
            category_id: "cat-1".to_owned(),
            created_at: chrono::Utc::now(),
        },
        CategorySummary {
            object_id: "oid-2".to_owned(),
            display_name: "Random".to_owned(),
            description: String::new(),
            category_id: "cat-2".to_owned(),
            created_at: chrono::Utc::now(),
        },
    ];

    let state = state
        .apply(Action::ComposeAppend('h'))
        .apply(Action::ComposeAppend('i'))
        .apply(Action::ComposeBackspace)
        .apply(Action::ComposeTogglePreview)
        .apply(Action::MoveComposeCategory(-1));

    assert_eq!(state.compose.text, "h");
    assert!(state.compose.preview);
    assert_eq!(state.compose.category_index, 1);
}

#[test]
fn toggling_selected_subscription_adds_and_removes_the_selected_category() {
    let mut state = AppState::new();
    state.screen = Screen::Subscriptions;
    state.categories = vec![CategorySummary {
        object_id: "oid-1".to_owned(),
        display_name: "General".to_owned(),
        description: String::new(),
        category_id: "cat-1".to_owned(),
        created_at: chrono::Utc::now(),
    }];

    let state = state.apply(Action::ToggleSelectedSubscription);
    assert_eq!(state.subscriptions.category_ids, vec!["cat-1".to_owned()]);

    let state = state.apply(Action::ToggleSelectedSubscription);
    assert!(state.subscriptions.category_ids.is_empty());
}

#[test]
fn feed_selection_helpers_use_only_subscribed_categories() {
    let mut state = AppState::new();
    state.categories = vec![
        CategorySummary {
            object_id: "oid-1".to_owned(),
            display_name: "General".to_owned(),
            description: String::new(),
            category_id: "cat-1".to_owned(),
            created_at: chrono::Utc::now(),
        },
        CategorySummary {
            object_id: "oid-2".to_owned(),
            display_name: "Random".to_owned(),
            description: String::new(),
            category_id: "cat-2".to_owned(),
            created_at: chrono::Utc::now(),
        },
    ];
    state.subscriptions.category_ids = vec!["cat-2".to_owned()];
    state.selected_category_index = 5;
    state.selected_post_index = 9;

    state.clamp_feed_post_index();

    assert_eq!(state.visible_feed_category_count(), 1);
    assert_eq!(state.selected_category_index, 0);
    assert_eq!(
        state
            .selected_feed_category()
            .map(|category| category.category_id.as_str()),
        Some("cat-2")
    );
    assert_eq!(state.selected_post_index, 0);
}
