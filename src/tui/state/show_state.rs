pub struct ShowState {
    pub scroll_offset: u16,
}

impl ShowState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
        }
    }
}