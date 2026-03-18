use crate::cli::Target;
use crate::converter;
use crate::ir::ThemeIR;
use crate::reader::{ThemeEntry, ThemeReader};
use crate::store;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const PAGE_SIZE: usize = 10;

/// Find theme labels that appear more than once (case-sensitive).
fn duplicate_labels(themes: &[ThemeEntry]) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut dupes = HashSet::new();
    for t in themes {
        if !seen.insert(&t.label) {
            dupes.insert(t.label.clone());
        }
    }
    dupes
}

pub struct PreviewApp<'a> {
    all_themes: Vec<ThemeEntry>,
    active_id: Option<String>,
    reader: &'a ThemeReader,
    target: &'a Target,
    selected: usize,
    filter: String,
    filtered_indices: Vec<usize>,
    ir_cache: HashMap<PathBuf, Result<ThemeIR, String>>,
    saved_slugs: HashSet<String>,
}

impl<'a> PreviewApp<'a> {
    pub fn new(
        themes: Vec<ThemeEntry>,
        active_id: Option<String>,
        reader: &'a ThemeReader,
        target: &'a Target,
        saved_slugs: HashSet<String>,
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
            saved_slugs,
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

    /// Get filtered theme labels (appends extension name for duplicates).
    pub fn filtered_labels(&self) -> Vec<String> {
        let dupes = duplicate_labels(&self.all_themes);
        self.filtered_indices
            .iter()
            .map(|&i| {
                let t = &self.all_themes[i];
                if dupes.contains(&t.label) {
                    format!("{} ({})", t.label, t.extension_name)
                } else {
                    t.label.clone()
                }
            })
            .collect()
    }

    /// Get filtered theme settings IDs.
    pub fn filtered_settings_ids(&self) -> Vec<String> {
        self.filtered_indices
            .iter()
            .map(|&i| self.all_themes[i].settings_id.clone())
            .collect()
    }

    /// Get saved flags for filtered themes (true if IR JSON exists in ~/chromaport/themes/).
    pub fn filtered_saved_flags(&self) -> Vec<bool> {
        self.filtered_indices
            .iter()
            .map(|&i| {
                let slug = store::theme_slug(&self.all_themes[i].label);
                self.saved_slugs.contains(&slug)
            })
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

    pub fn move_page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
    }

    pub fn move_page_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + PAGE_SIZE).min(self.filtered_indices.len() - 1);
        }
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Target;

    /// Create a minimal ThemeReader for testing (never actually reads files).
    fn make_test_reader() -> ThemeReader {
        ThemeReader::new(
            PathBuf::from("/tmp/ext"),
            PathBuf::from("/tmp/settings.json"),
        )
    }

    /// Create N dummy ThemeEntry items for testing navigation.
    fn make_entries(n: usize) -> Vec<ThemeEntry> {
        (0..n)
            .map(|i| ThemeEntry {
                label: format!("Theme {i}"),
                settings_id: format!("theme-{i}"),
                ui_theme: "vs-dark".to_string(),
                path: PathBuf::from(format!("/tmp/theme-{i}.json")),
                extension_name: "test-ext".to_string(),
            })
            .collect()
    }

    fn make_app(n: usize) -> PreviewApp<'static> {
        // Leak to get 'static lifetime for test convenience
        let reader: &'static ThemeReader = Box::leak(Box::new(make_test_reader()));
        let target: &'static Target = Box::leak(Box::new(Target::Ghostty));
        PreviewApp::new(make_entries(n), None, reader, target, HashSet::new())
    }

    #[test]
    fn move_page_down_advances_by_page_size() {
        let mut app = make_app(30);
        assert_eq!(app.selected_index(), 0);
        app.move_page_down();
        assert_eq!(app.selected_index(), 10);
        app.move_page_down();
        assert_eq!(app.selected_index(), 20);
    }

    #[test]
    fn move_page_down_clamps_to_last() {
        let mut app = make_app(15);
        app.move_page_down();
        assert_eq!(app.selected_index(), 10);
        app.move_page_down();
        assert_eq!(app.selected_index(), 14); // last item
    }

    #[test]
    fn move_page_up_goes_back() {
        let mut app = make_app(30);
        app.move_to_bottom();
        assert_eq!(app.selected_index(), 29);
        app.move_page_up();
        assert_eq!(app.selected_index(), 19);
    }

    #[test]
    fn move_page_up_clamps_to_zero() {
        let mut app = make_app(15);
        app.move_page_up(); // already at 0
        assert_eq!(app.selected_index(), 0);
    }

    #[test]
    fn move_to_top_and_bottom() {
        let mut app = make_app(20);
        app.move_to_bottom();
        assert_eq!(app.selected_index(), 19);
        app.move_to_top();
        assert_eq!(app.selected_index(), 0);
    }

    #[test]
    fn navigation_on_empty_list_is_safe() {
        let mut app = make_app(0);
        app.move_page_down();
        app.move_page_up();
        app.move_to_top();
        app.move_to_bottom();
        assert_eq!(app.selected_index(), 0);
    }
}
