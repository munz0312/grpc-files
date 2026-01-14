use crate::fileservice::FileInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Uploading,
    CreatingDirectory,
}

pub struct App {
    files: Vec<FileInfo>,
    selected_index: usize,
    status_message: Option<String>,
    mode: AppMode,
    selected_file_path: Option<String>,
    current_directory: String,
}

impl App {
    pub fn new() -> Self {
        App {
            files: Vec::new(),
            selected_index: 0,
            status_message: Some("Press r to refresh".to_string()),
            mode: AppMode::Normal,
            selected_file_path: None,
            current_directory: String::new(),
        }
    }

    pub fn set_current_directory(&mut self, path: String) {
        self.current_directory = path;
    }

    pub fn current_directory(&self) -> &str {
        &self.current_directory
    }

    pub fn is_at_root(&self) -> bool {
        self.current_directory.is_empty()
    }

    pub fn parent_directory_path(&self) -> Option<String> {
        if self.current_directory.is_empty() {
            None
        } else {
            // Find the last slash and return everything before it
            if let Some(last_slash) = self.current_directory.rfind('/') {
                Some(self.current_directory[..last_slash].to_string())
            } else {
                Some(String::new())
            }
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

    pub fn update_files(&mut self, new_files: Vec<FileInfo>, current_path: String) {
        self.current_directory = current_path.clone();

        // Add parent directory entry if not at root
        let mut files = new_files;
        if !current_path.is_empty() {
            let parent_path = if let Some(last_slash) = current_path.rfind('/') {
                current_path[..last_slash].to_string()
            } else {
                String::new()
            };
            files.insert(0, FileInfo {
                filename: "..".to_string(),
                size: 0,
                upload_time: None,
                is_directory: true,
                path: parent_path,
            });
        }

        self.files = files;
        if self.selected_index >= self.files.len() {
            self.selected_index = self.files.len().saturating_sub(1);
        }
    }

    pub fn enter_directory(&mut self) -> Option<String> {
        if let Some(file) = self.selected_file() {
            if file.is_directory {
                return Some(file.path.clone());
            }
        }
        None
    }

    pub fn selected_is_directory(&self) -> bool {
        self.selected_file()
            .map(|f| f.is_directory)
            .unwrap_or(false)
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
