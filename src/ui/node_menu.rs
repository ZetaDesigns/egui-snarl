use egui::{Atom, Color32, Key, RichText, ScrollArea, TextEdit, Ui};

/// One selectable node type in a [`NodeMenuCategory`].
#[derive(Clone, Debug)]
pub struct NodeMenuEntry<'a, T> {
    label: &'a str,
    keywords: &'a str,
    value: T,
}

impl<'a, T> NodeMenuEntry<'a, T> {
    /// Creates a node menu entry with the label used for display and search.
    pub const fn new(label: &'a str, value: T) -> Self {
        Self {
            label,
            keywords: "",
            value,
        }
    }

    /// Adds extra whitespace-separated text that can find this entry in search.
    #[must_use]
    pub const fn with_keywords(mut self, keywords: &'a str) -> Self {
        self.keywords = keywords;
        self
    }

    /// Returns the displayed entry label.
    #[must_use]
    pub const fn label(&self) -> &str {
        self.label
    }

    /// Returns the application-defined value represented by this entry.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }
}

/// A named group of node types shown as one submenu in [`NodeMenu`].
#[derive(Clone, Debug)]
pub struct NodeMenuCategory<'a, T> {
    label: &'a str,
    color: Color32,
    entries: &'a [NodeMenuEntry<'a, T>],
}

impl<'a, T> NodeMenuCategory<'a, T> {
    /// Creates a category with a shared identifying color.
    pub const fn new(label: &'a str, color: Color32, entries: &'a [NodeMenuEntry<'a, T>]) -> Self {
        Self {
            label,
            color,
            entries,
        }
    }

    /// Returns the displayed category label.
    #[must_use]
    pub const fn label(&self) -> &str {
        self.label
    }

    /// Returns the color associated with all entries in this category.
    #[must_use]
    pub const fn color(&self) -> Color32 {
        self.color
    }

    /// Returns the node entries in this category.
    #[must_use]
    pub const fn entries(&self) -> &[NodeMenuEntry<'a, T>] {
        self.entries
    }
}

/// Display options for a [`NodeMenu`].
#[derive(Clone, Copy, Debug)]
pub struct NodeMenuOptions<'a> {
    search_hint: &'a str,
    no_matches: &'a str,
    min_width: f32,
    max_search_height: f32,
}

impl<'a> NodeMenuOptions<'a> {
    /// Creates options with application-provided, localizable search text.
    #[must_use]
    pub const fn new(search_hint: &'a str, no_matches: &'a str) -> Self {
        Self {
            search_hint,
            no_matches,
            min_width: 240.0,
            max_search_height: 320.0,
        }
    }

    /// Sets the minimum width of the root menu and category submenus.
    #[must_use]
    pub const fn with_min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Sets the maximum height of the scrollable search results.
    #[must_use]
    pub const fn with_max_search_height(mut self, max_search_height: f32) -> Self {
        self.max_search_height = max_search_height;
        self
    }
}

impl Default for NodeMenuOptions<'_> {
    fn default() -> Self {
        Self::new("Search nodes…", "No matching nodes")
    }
}

/// Stateful, Blender-style node selector for use inside a graph context menu.
///
/// With an empty search it shows only categories at the top level. Typing in
/// the search field replaces those categories with matching node types. Store
/// one menu alongside the [`crate::ui::SnarlViewer`] that renders it so its
/// search text survives across UI frames while the context menu is open.
#[derive(Clone, Debug, Default)]
pub struct NodeMenu {
    search: String,
    last_frame: Option<u64>,
}

impl NodeMenu {
    /// Creates an empty node menu.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            search: String::new(),
            last_frame: None,
        }
    }

    /// Returns the current search text.
    #[must_use]
    pub fn search(&self) -> &str {
        &self.search
    }

    /// Clears the current search text.
    pub fn clear_search(&mut self) {
        self.search.clear();
    }

    /// Shows the selector and returns the value of the clicked node entry.
    ///
    /// Category labels and entry keywords participate in search in addition to
    /// the displayed node label. Search is case-insensitive and every
    /// whitespace-separated term must match.
    pub fn show<'a, T>(
        &mut self,
        ui: &mut Ui,
        categories: &'a [NodeMenuCategory<'a, T>],
        options: NodeMenuOptions<'_>,
    ) -> Option<&'a T> {
        let frame = ui.ctx().cumulative_frame_nr();
        let newly_opened = self
            .last_frame
            .is_none_or(|last_frame| frame > last_frame.saturating_add(1));
        if newly_opened {
            self.search.clear();
        }
        self.last_frame = Some(frame);

        ui.set_min_width(options.min_width);
        let search_response = ui.add(
            TextEdit::singleline(&mut self.search)
                .hint_text(options.search_hint.to_owned())
                .desired_width(f32::INFINITY),
        );
        if newly_opened {
            search_response.request_focus();
        }
        let submit_search =
            search_response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        ui.separator();

        let query = self.search.trim().to_lowercase();
        let mut selected = None;

        if query.is_empty() {
            let mut category_count = 0;
            for (category_index, category) in categories.iter().enumerate() {
                if category.entries.is_empty() {
                    continue;
                }
                category_count += 1;
                ui.push_id(category_index, |ui| {
                    ui.menu_button(
                        (RichText::new("●").color(category.color), category.label),
                        |ui| {
                            ui.set_min_width(options.min_width);
                            for (entry_index, entry) in category.entries.iter().enumerate() {
                                ui.push_id(entry_index, |ui| {
                                    if ui.button(entry.label).clicked() {
                                        selected = Some((category_index, entry_index));
                                        ui.close();
                                    }
                                });
                            }
                        },
                    );
                });
            }
            if category_count == 0 {
                ui.label(RichText::new(options.no_matches).weak());
            }
        } else {
            let mut first_match = None;
            let mut match_count = 0;
            ScrollArea::vertical()
                .max_height(options.max_search_height)
                .show(ui, |ui| {
                    for (category_index, category) in categories.iter().enumerate() {
                        for (entry_index, entry) in category.entries.iter().enumerate() {
                            if !entry_matches(entry, category.label, &query) {
                                continue;
                            }
                            let location = (category_index, entry_index);
                            first_match.get_or_insert(location);
                            match_count += 1;
                            ui.push_id(location, |ui| {
                                if ui
                                    .button((
                                        RichText::new("●").color(category.color),
                                        entry.label,
                                        Atom::grow(),
                                        RichText::new(category.label).small().weak(),
                                    ))
                                    .clicked()
                                {
                                    selected = Some(location);
                                    ui.close();
                                }
                            });
                        }
                    }
                });

            if match_count == 0 {
                ui.label(RichText::new(options.no_matches).weak());
            } else if submit_search && selected.is_none() {
                selected = first_match;
                ui.close();
            }
        }

        let value = selected.map(|(category, entry)| &categories[category].entries[entry].value);
        if value.is_some() {
            self.search.clear();
        }
        value
    }
}

fn entry_matches<T>(entry: &NodeMenuEntry<'_, T>, category: &str, query: &str) -> bool {
    let label = entry.label.to_lowercase();
    let category = category.to_lowercase();
    let keywords = entry.keywords.to_lowercase();
    let query = query.to_lowercase();
    query
        .split_whitespace()
        .all(|term| label.contains(term) || category.contains(term) || keywords.contains(term))
}

#[cfg(test)]
mod tests {
    use super::{NodeMenuEntry, entry_matches};

    #[test]
    fn node_search_is_case_insensitive_and_matches_all_terms() {
        let entry = NodeMenuEntry::new("Gradient Map", ()).with_keywords("tone colorize");

        assert!(entry_matches(&entry, "Adjustments", "gradient"));
        assert!(entry_matches(&entry, "Adjustments", "ADJUSTMENTS map"));
        assert!(entry_matches(&entry, "Adjustments", "tone map"));
        assert!(!entry_matches(&entry, "Adjustments", "gradient mask"));
    }
}
