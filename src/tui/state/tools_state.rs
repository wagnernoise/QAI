use ratatui::widgets::ListState;

pub struct ToolsState {
    pub tools_provider_index: usize,
    pub tools_provider_list_state: ListState,
}

impl ToolsState {
    pub fn new() -> Self {
        let mut tools_provider_list_state = ListState::default();
        tools_provider_list_state.select(Some(0));
        Self {
            tools_provider_index: 0,
            tools_provider_list_state,
        }
    }
}