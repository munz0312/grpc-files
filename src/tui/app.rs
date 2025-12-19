use crate::fileservice::FileInfo;

pub struct App {
    files: Vec<FileInfo>,
    selected_index: usize,
    status_message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        App {
            files: Vec::new(),
            selected_index: 0,
            status_message: Some("Press r to refresh".to_string()),
        }
    }

    pub fn select_next(&mut self) {
        if !self.files.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.files.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.files.is_empty() {
            match self.selected_index {
                0 => self.selected_index = self.files.len() - 1,
                _ => self.selected_index -= 1,
            }
        }
    }

    pub fn update_files(&mut self, new_files: Vec<FileInfo>) {
        self.files = new_files;
        if self.selected_index > self.files.len() {
            self.selected_index = self.files.len() - 1;
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }
}
