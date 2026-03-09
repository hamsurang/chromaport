use crate::cli::Target;
use crate::converter;
use crate::ir::ThemeIR;
use crate::reader::{ThemeEntry, ThemeReader};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PreviewApp<'a> {
    all_themes: Vec<ThemeEntry>,
    active_id: Option<String>,
    reader: &'a ThemeReader,
    target: &'a Target,
    selected: usize,
    filter: String,
    filtered_indices: Vec<usize>,
    ir_cache: HashMap<PathBuf, Result<ThemeIR, String>>,
}

impl<'a> PreviewApp<'a> {
    pub fn new(
        themes: Vec<ThemeEntry>,
        active_id: Option<String>,
        reader: &'a ThemeReader,
        target: &'a Target,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..themes.len()).collect();
        Self {
            all_themes: themes,
            active_id,
            reader,
            target,
            selected: 0,
            filter: String::new(),
            filtered_indices,
            ir_cache: HashMap::new(),
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn target(&self) -> &Target {
        self.target
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }

    /// Get filtered theme labels.
    pub fn filtered_labels(&self) -> Vec<String> {
        self.filtered_indices
            .iter()
            .map(|&i| self.all_themes[i].label.clone())
            .collect()
    }

    /// Get filtered theme settings IDs.
    pub fn filtered_settings_ids(&self) -> Vec<String> {
        self.filtered_indices
            .iter()
            .map(|&i| self.all_themes[i].settings_id.clone())
            .collect()
    }

    fn current_entry(&self) -> Option<&ThemeEntry> {
        self.filtered_indices
            .get(self.selected)
            .map(|&i| &self.all_themes[i])
    }

    /// Ensure the current theme's IR is in the cache (stores errors too).
    pub fn ensure_current_cached(&mut self) {
        if let Some(entry) = self.current_entry().cloned() {
            if !self.ir_cache.contains_key(&entry.path) {
                let result = self
                    .reader
                    .read_theme_json(&entry)
                    .and_then(|json| converter::convert(&entry, &json))
                    .map_err(|e| format!("{e:#}"));
                self.ir_cache.insert(entry.path, result);
            }
        }
    }

    /// Get the cached ThemeIR for the current selection (immutable).
    pub fn cached_current_ir(&self) -> Option<&ThemeIR> {
        self.current_entry()
            .and_then(|e| self.ir_cache.get(&e.path))
            .and_then(|r| r.as_ref().ok())
    }

    /// Get the cached error for the current selection, if conversion failed.
    pub fn cached_current_error(&self) -> Option<&str> {
        self.current_entry()
            .and_then(|e| self.ir_cache.get(&e.path))
            .and_then(|r| r.as_ref().err())
            .map(|s| s.as_str())
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn select(&self) -> Option<ThemeEntry> {
        self.current_entry().cloned()
    }

    pub fn add_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.update_filter();
    }

    pub fn delete_filter_char(&mut self) {
        self.filter.pop();
        self.update_filter();
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.update_filter();
    }

    fn update_filter(&mut self) {
        let lower_filter = self.filter.to_lowercase();
        self.filtered_indices = self
            .all_themes
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                lower_filter.is_empty() || t.label.to_lowercase().contains(&lower_filter)
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }
}
