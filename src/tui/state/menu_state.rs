use ratatui::widgets::ListState;

pub struct MenuState {
    pub menu_state: ListState,
}

impl MenuState {
    pub fn new() -> Self {
        let mut menu_state = ListState::default();
        menu_state.select(Some(0));
        Self { menu_state }
    }
}