use crate::fileservice::FileInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Uploading,
}

pub struct App {
    files: Vec<FileInfo>,
    selected_index: usize,
    status_message: Option<String>,
    mode: AppMode,
    selected_file_path: Option<String>,
}

impl App {
    pub fn new() -> Self {
        App {
            files: Vec::new(),
            selected_index: 0,
            status_message: Some("Press r to refresh".to_string()),
            mode: AppMode::Normal,
            selected_file_path: None,
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

    pub fn files(&self) -> &Vec<FileInfo> {
        &self.files
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn status_message(&self) -> &Option<String> {
        &self.status_message
    }

    pub fn selected_file(&self) -> Option<&FileInfo> {
        self.files.get(self.selected_index)
    }

    pub fn set_file_for_upload(&mut self, path: String) {
        self.selected_file_path = Some(path);
    }

    pub fn clear_file_path(&mut self) {
        self.selected_file_path = None;
    }

    pub fn selected_file_path(&self) -> &Option<String> {
        &self.selected_file_path
    }

    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
    }

    pub fn is_uploading(&self) -> bool {
        matches!(self.mode, AppMode::Uploading)
    }
}
