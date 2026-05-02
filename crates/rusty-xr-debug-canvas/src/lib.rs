//! Dependency-light logical canvas primitives for headset diagnostics.
//!
//! The crate emits normalized rectangles and text runs. App shells decide how to
//! render them with their own UI, Vulkan, OpenXR, terminal, or image backends.

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Semantic color role for a canvas primitive.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasTone {
    Surface,
    Header,
    Band,
    Border,
    Text,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
    Shadow,
}

/// Text role for renderers that can vary font size or weight.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasTextRole {
    Title,
    Section,
    Label,
    Body,
    Small,
}

/// Compact color theme for test and diagnostics panels.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTheme {
    pub surface: [f32; 4],
    pub header: [f32; 4],
    pub band: [f32; 4],
    pub border: [f32; 4],
    pub text: [f32; 4],
    pub muted: [f32; 4],
    pub accent: [f32; 4],
    pub success: [f32; 4],
    pub warning: [f32; 4],
    pub danger: [f32; 4],
    pub shadow: [f32; 4],
}

impl Default for CanvasTheme {
    fn default() -> Self {
        Self {
            surface: [0.086, 0.137, 0.192, 0.93],
            header: [0.063, 0.106, 0.149, 0.94],
            band: [0.051, 0.094, 0.141, 0.88],
            border: [0.300, 0.470, 0.610, 0.58],
            text: [0.937, 0.969, 1.000, 1.00],
            muted: [0.720, 0.784, 0.847, 1.00],
            accent: [0.620, 0.800, 1.000, 1.00],
            success: [0.520, 0.920, 0.660, 1.00],
            warning: [1.000, 0.760, 0.360, 1.00],
            danger: [1.000, 0.440, 0.400, 1.00],
            shadow: [0.000, 0.000, 0.000, 0.28],
        }
    }
}

impl CanvasTheme {
    pub const fn color_for(self, tone: CanvasTone) -> [f32; 4] {
        match tone {
            CanvasTone::Surface => self.surface,
            CanvasTone::Header => self.header,
            CanvasTone::Band => self.band,
            CanvasTone::Border => self.border,
            CanvasTone::Text => self.text,
            CanvasTone::Muted => self.muted,
            CanvasTone::Accent => self.accent,
            CanvasTone::Success => self.success,
            CanvasTone::Warning => self.warning,
            CanvasTone::Danger => self.danger,
            CanvasTone::Shadow => self.shadow,
        }
    }
}

/// Grid and spacing policy for a normalized debug canvas.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasLayout {
    pub columns: usize,
    pub rows: usize,
    pub padding_columns: usize,
    pub header_rows: usize,
    pub key_columns: usize,
}

impl Default for CanvasLayout {
    fn default() -> Self {
        Self {
            columns: 86,
            rows: 30,
            padding_columns: 3,
            header_rows: 4,
            key_columns: 13,
        }
    }
}

impl CanvasLayout {
    /// Build normalized draw primitives for a document.
    pub fn layout_document(self, document: &CanvasDocument, theme: &CanvasTheme) -> CanvasDrawList {
        let columns = self.columns.max(12);
        let rows = self.rows.max(8);
        let padding = self.padding_columns.min(columns / 4).max(1);
        let header_rows = self.header_rows.clamp(1, rows.saturating_sub(3));
        let mut cursor = header_rows;
        let mut list = CanvasDrawList {
            columns,
            rows,
            rects: Vec::new(),
            text: Vec::new(),
            shadow_color: theme.color_for(CanvasTone::Shadow),
            overflowed: false,
        };

        list.rects.push(CanvasRect::new(
            [0.0, 0.0, 1.0, 1.0],
            CanvasTone::Surface,
            theme,
        ));
        list.rects.push(CanvasRect::new(
            self.cell_rect(columns, rows, 0, 0, columns, header_rows),
            CanvasTone::Header,
            theme,
        ));
        self.push_border(&mut list, theme);

        list.push_text(
            self.text_run(
                columns,
                rows,
                padding,
                1.min(rows - 1),
                columns - padding * 2,
                1,
            ),
            clamp_text(&document.title, columns - padding * 2),
            CanvasTone::Accent,
            CanvasTextRole::Title,
            theme,
        );
        if let Some(subtitle) = &document.subtitle {
            if header_rows > 2 {
                list.push_text(
                    self.text_run(columns, rows, padding, 2, columns - padding * 2, 1),
                    clamp_text(subtitle, columns - padding * 2),
                    CanvasTone::Muted,
                    CanvasTextRole::Small,
                    theme,
                );
            }
        }

        for section in &document.sections {
            if cursor >= rows.saturating_sub(1) {
                list.overflowed = true;
                break;
            }
            self.layout_section(section, &mut list, &mut cursor, theme);
        }

        if let Some(footer) = &document.footer {
            let footer_row = rows.saturating_sub(2);
            if footer_row > header_rows && !footer.is_empty() {
                list.push_text(
                    self.text_run(columns, rows, padding, footer_row, columns - padding * 2, 1),
                    clamp_text(footer, columns - padding * 2),
                    CanvasTone::Muted,
                    CanvasTextRole::Small,
                    theme,
                );
            }
        }
        if list.overflowed {
            list.push_text(
                self.text_run(columns, rows, columns.saturating_sub(5), rows - 2, 3, 1),
                "...".to_string(),
                CanvasTone::Warning,
                CanvasTextRole::Small,
                theme,
            );
        }

        list
    }

    fn layout_section(
        self,
        section: &CanvasSection,
        list: &mut CanvasDrawList,
        cursor: &mut usize,
        theme: &CanvasTheme,
    ) {
        let columns = list.columns;
        let rows = list.rows;
        let padding = self.padding_columns.min(columns / 4).max(1);
        let body_width = columns - padding * 2;

        if let Some(title) = &section.title {
            if !self.try_rows(list, *cursor, 1) {
                return;
            }
            list.rects.push(CanvasRect::new(
                self.cell_rect(
                    columns,
                    rows,
                    padding.saturating_sub(1),
                    *cursor,
                    body_width + 2,
                    1,
                ),
                CanvasTone::Band,
                theme,
            ));
            list.push_text(
                self.text_run(columns, rows, padding, *cursor, body_width, 1),
                clamp_text(title, body_width),
                CanvasTone::Accent,
                CanvasTextRole::Section,
                theme,
            );
            *cursor += 1;
        }

        for row in &section.rows {
            if *cursor >= rows.saturating_sub(1) {
                list.overflowed = true;
                return;
            }
            match row {
                CanvasRow::Text { text, tone } => {
                    let lines = wrap_text(text, body_width, rows.saturating_sub(*cursor + 1));
                    if lines.is_empty() {
                        continue;
                    }
                    for line in lines {
                        if !self.try_rows(list, *cursor, 1) {
                            return;
                        }
                        list.push_text(
                            self.text_run(columns, rows, padding, *cursor, body_width, 1),
                            line,
                            *tone,
                            CanvasTextRole::Body,
                            theme,
                        );
                        *cursor += 1;
                    }
                }
                CanvasRow::KeyValue { key, value, tone } => {
                    let key_width = self.key_columns.min(body_width.saturating_sub(8)).max(5);
                    let value_col = padding + key_width + 1;
                    let value_width = columns.saturating_sub(value_col + padding).max(4);
                    let lines = wrap_text(value, value_width, rows.saturating_sub(*cursor + 1));
                    let lines = if lines.is_empty() {
                        vec![String::new()]
                    } else {
                        lines
                    };
                    for (index, line) in lines.into_iter().enumerate() {
                        if !self.try_rows(list, *cursor, 1) {
                            return;
                        }
                        if index == 0 {
                            list.push_text(
                                self.text_run(columns, rows, padding, *cursor, key_width, 1),
                                clamp_text(key, key_width),
                                CanvasTone::Muted,
                                CanvasTextRole::Label,
                                theme,
                            );
                        }
                        list.push_text(
                            self.text_run(columns, rows, value_col, *cursor, value_width, 1),
                            line,
                            *tone,
                            CanvasTextRole::Body,
                            theme,
                        );
                        *cursor += 1;
                    }
                }
                CanvasRow::Badges(badges) => {
                    self.layout_badges(badges, list, cursor, theme);
                }
                CanvasRow::Rule => {
                    if !self.try_rows(list, *cursor, 1) {
                        return;
                    }
                    let mut rect = self.cell_rect(columns, rows, padding, *cursor, body_width, 1);
                    rect[1] += rect[3] * 0.48;
                    rect[3] *= 0.08;
                    list.rects
                        .push(CanvasRect::new(rect, CanvasTone::Border, theme));
                    *cursor += 1;
                }
            }
        }
    }

    fn layout_badges(
        self,
        badges: &[CanvasBadge],
        list: &mut CanvasDrawList,
        cursor: &mut usize,
        theme: &CanvasTheme,
    ) {
        let columns = list.columns;
        let rows = list.rows;
        let padding = self.padding_columns.min(columns / 4).max(1);
        let right_edge = columns.saturating_sub(padding);
        let mut col = padding;
        for badge in badges {
            let text = badge.display_text();
            let width = text
                .chars()
                .count()
                .saturating_add(2)
                .clamp(6, right_edge - padding);
            if col + width > right_edge {
                *cursor += 1;
                col = padding;
            }
            if !self.try_rows(list, *cursor, 1) {
                return;
            }
            list.rects.push(CanvasRect::new(
                self.cell_rect(columns, rows, col, *cursor, width, 1),
                CanvasTone::Band,
                theme,
            ));
            list.push_text(
                self.text_run(columns, rows, col + 1, *cursor, width.saturating_sub(2), 1),
                clamp_text(&text, width.saturating_sub(2)),
                badge.tone,
                CanvasTextRole::Small,
                theme,
            );
            col += width + 1;
        }
        *cursor += 1;
    }

    fn try_rows(self, list: &mut CanvasDrawList, cursor: usize, count: usize) -> bool {
        let ok = cursor.saturating_add(count) < list.rows;
        if !ok {
            list.overflowed = true;
        }
        ok
    }

    fn push_border(self, list: &mut CanvasDrawList, theme: &CanvasTheme) {
        let columns = list.columns as f32;
        let rows = list.rows as f32;
        let border_x = (0.45 / columns).max(0.004);
        let border_y = (0.45 / rows).max(0.006);
        for rect in [
            [0.0, 0.0, 1.0, border_y],
            [0.0, 1.0 - border_y, 1.0, border_y],
            [0.0, 0.0, border_x, 1.0],
            [1.0 - border_x, 0.0, border_x, 1.0],
        ] {
            list.rects
                .push(CanvasRect::new(rect, CanvasTone::Border, theme));
        }
    }

    fn cell_rect(
        self,
        columns: usize,
        rows: usize,
        col: usize,
        row: usize,
        width: usize,
        height: usize,
    ) -> [f32; 4] {
        [
            col as f32 / columns as f32,
            row as f32 / rows as f32,
            width as f32 / columns as f32,
            height as f32 / rows as f32,
        ]
    }

    fn text_run(
        self,
        columns: usize,
        rows: usize,
        col: usize,
        row: usize,
        width: usize,
        height: usize,
    ) -> [f32; 4] {
        self.cell_rect(columns, rows, col, row, width, height)
    }
}

/// A render-target-neutral canvas document.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasDocument {
    pub title: String,
    pub subtitle: Option<String>,
    pub sections: Vec<CanvasSection>,
    pub footer: Option<String>,
}

impl CanvasDocument {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            sections: Vec::new(),
            footer: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn with_section(mut self, section: CanvasSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

/// A named group of canvas rows.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasSection {
    pub title: Option<String>,
    pub rows: Vec<CanvasRow>,
}

impl CanvasSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            rows: Vec::new(),
        }
    }

    pub fn unnamed() -> Self {
        Self {
            title: None,
            rows: Vec::new(),
        }
    }

    pub fn with_text(mut self, text: impl Into<String>, tone: CanvasTone) -> Self {
        self.rows.push(CanvasRow::Text {
            text: text.into(),
            tone,
        });
        self
    }

    pub fn with_key_value(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
        tone: CanvasTone,
    ) -> Self {
        self.rows.push(CanvasRow::KeyValue {
            key: key.into(),
            value: value.into(),
            tone,
        });
        self
    }

    pub fn with_badges(mut self, badges: Vec<CanvasBadge>) -> Self {
        self.rows.push(CanvasRow::Badges(badges));
        self
    }

    pub fn with_rule(mut self) -> Self {
        self.rows.push(CanvasRow::Rule);
        self
    }
}

/// Row content for a canvas section.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasRow {
    Text {
        text: String,
        tone: CanvasTone,
    },
    KeyValue {
        key: String,
        value: String,
        tone: CanvasTone,
    },
    Badges(Vec<CanvasBadge>),
    Rule,
}

/// Small labeled status pill.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasBadge {
    pub label: String,
    pub value: String,
    pub tone: CanvasTone,
}

impl CanvasBadge {
    pub fn new(label: impl Into<String>, value: impl Into<String>, tone: CanvasTone) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone,
        }
    }

    pub fn display_text(&self) -> String {
        if self.value.is_empty() {
            self.label.clone()
        } else {
            format!("{} {}", self.label, self.value)
        }
    }
}

/// Output of a canvas layout pass.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasDrawList {
    pub columns: usize,
    pub rows: usize,
    pub rects: Vec<CanvasRect>,
    pub text: Vec<CanvasTextRun>,
    pub shadow_color: [f32; 4],
    pub overflowed: bool,
}

impl CanvasDrawList {
    fn push_text(
        &mut self,
        rect: [f32; 4],
        text: String,
        tone: CanvasTone,
        role: CanvasTextRole,
        theme: &CanvasTheme,
    ) {
        if text.is_empty() {
            return;
        }
        let columns = ((rect[2] * self.columns as f32).round() as usize).max(1);
        self.text.push(CanvasTextRun {
            rect,
            columns,
            text,
            tone,
            role,
            color: theme.color_for(tone),
        });
    }
}

/// Normalized filled rectangle.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub rect: [f32; 4],
    pub tone: CanvasTone,
    pub color: [f32; 4],
}

impl CanvasRect {
    pub fn new(rect: [f32; 4], tone: CanvasTone, theme: &CanvasTheme) -> Self {
        Self {
            rect,
            tone,
            color: theme.color_for(tone),
        }
    }
}

/// One line of text in normalized canvas coordinates.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasTextRun {
    pub rect: [f32; 4],
    pub columns: usize,
    pub text: String,
    pub tone: CanvasTone,
    pub role: CanvasTextRole,
    pub color: [f32; 4],
}

/// Where a diagnostic HUD command came from.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticHudInputSource {
    RuntimeConfig,
    AdbIntent,
    Controller,
    Lsl,
    Osc,
    Application,
}

impl DiagnosticHudInputSource {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::RuntimeConfig => "runtime-config",
            Self::AdbIntent => "adb-intent",
            Self::Controller => "controller",
            Self::Lsl => "lsl",
            Self::Osc => "osc",
            Self::Application => "application",
        }
    }
}

/// Input-neutral command vocabulary for diagnostic HUD adapters.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticHudCommand {
    Show,
    Hide,
    Toggle,
    SetVisible(bool),
    NextPage,
    PreviousPage,
    SetPage(usize),
}

/// Snapshot returned after a HUD command is applied.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticHudUpdate {
    pub changed: bool,
    pub visible: bool,
    pub page_index: usize,
    pub page_count: usize,
    pub revision: u64,
    pub last_input_source: Option<DiagnosticHudInputSource>,
}

/// Small, renderer-neutral visibility/page state for in-headset diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticHudState {
    visible: bool,
    page_index: usize,
    page_count: usize,
    revision: u64,
    last_input_source: Option<DiagnosticHudInputSource>,
}

impl Default for DiagnosticHudState {
    fn default() -> Self {
        Self::new(true)
    }
}

impl DiagnosticHudState {
    pub const fn new(visible: bool) -> Self {
        Self {
            visible,
            page_index: 0,
            page_count: 1,
            revision: 0,
            last_input_source: None,
        }
    }

    pub fn with_page_count(mut self, page_count: usize) -> Self {
        self.page_count = page_count.max(1);
        self.page_index = self.page_index.min(self.page_count - 1);
        self
    }

    pub const fn visible(self) -> bool {
        self.visible
    }

    pub const fn page_index(self) -> usize {
        self.page_index
    }

    pub const fn page_count(self) -> usize {
        self.page_count
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn last_input_source(self) -> Option<DiagnosticHudInputSource> {
        self.last_input_source
    }

    pub fn set_page_count(&mut self, page_count: usize) -> DiagnosticHudUpdate {
        let old = *self;
        self.page_count = page_count.max(1);
        self.page_index = self.page_index.min(self.page_count - 1);
        if old.page_count != self.page_count || old.page_index != self.page_index {
            self.revision = self.revision.saturating_add(1);
        }
        self.finish_update(old)
    }

    pub fn apply(
        &mut self,
        command: DiagnosticHudCommand,
        source: DiagnosticHudInputSource,
    ) -> DiagnosticHudUpdate {
        let old = *self;
        match command {
            DiagnosticHudCommand::Show => self.visible = true,
            DiagnosticHudCommand::Hide => self.visible = false,
            DiagnosticHudCommand::Toggle => self.visible = !self.visible,
            DiagnosticHudCommand::SetVisible(visible) => self.visible = visible,
            DiagnosticHudCommand::NextPage => {
                self.page_index = (self.page_index + 1) % self.page_count.max(1);
            }
            DiagnosticHudCommand::PreviousPage => {
                self.page_index = if self.page_index == 0 {
                    self.page_count.saturating_sub(1)
                } else {
                    self.page_index - 1
                };
            }
            DiagnosticHudCommand::SetPage(page) => {
                self.page_index = page.min(self.page_count.saturating_sub(1));
            }
        }
        self.last_input_source = Some(source);
        self.revision = self.revision.saturating_add(1);
        self.finish_update(old)
    }

    pub const fn snapshot(self) -> DiagnosticHudUpdate {
        DiagnosticHudUpdate {
            changed: false,
            visible: self.visible,
            page_index: self.page_index,
            page_count: self.page_count,
            revision: self.revision,
            last_input_source: self.last_input_source,
        }
    }

    fn finish_update(self, old: Self) -> DiagnosticHudUpdate {
        DiagnosticHudUpdate {
            changed: old.visible != self.visible
                || old.page_index != self.page_index
                || old.page_count != self.page_count,
            visible: self.visible,
            page_index: self.page_index,
            page_count: self.page_count,
            revision: self.revision,
            last_input_source: self.last_input_source,
        }
    }
}

fn clamp_text(value: &str, max_columns: usize) -> String {
    value
        .chars()
        .filter_map(clean_canvas_char)
        .take(max_columns)
        .collect()
}

fn wrap_text(value: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let words = value
        .split_whitespace()
        .map(|word| clamp_text(word, usize::MAX))
        .filter(|word| !word.is_empty());
    let mut lines = Vec::new();
    let mut current = String::new();

    for mut word in words {
        while word.chars().count() > width {
            if !current.is_empty() {
                lines.push(current);
                if lines.len() >= max_lines {
                    return lines;
                }
                current = String::new();
            }
            let chunk = word.chars().take(width).collect::<String>();
            word = word.chars().skip(width).collect::<String>();
            lines.push(chunk);
            if lines.len() >= max_lines {
                return lines;
            }
        }
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current.is_empty() {
            current.push_str(&word);
        } else if current_len + 1 + word_len <= width {
            current.push(' ');
            current.push_str(&word);
        } else {
            lines.push(current);
            if lines.len() >= max_lines {
                return lines;
            }
            current = word;
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines
}

fn clean_canvas_char(character: char) -> Option<char> {
    if character.is_ascii_graphic() || character == ' ' {
        Some(character)
    } else if character.is_whitespace() {
        Some(' ')
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn lays_out_primitives_inside_panel() {
        let document = CanvasDocument::new("OSC LIVE")
            .with_subtitle("GENERIC CONNECTOR")
            .with_section(CanvasSection::unnamed().with_badges(vec![
                CanvasBadge::new("STATUS", "LISTENING", CanvasTone::Success),
                CanvasBadge::new("PACKETS", "12", CanvasTone::Accent),
            ]))
            .with_section(
                CanvasSection::new("LAST PACKET")
                    .with_key_value("ADDRESS", "/rusty-xr/probe", CanvasTone::Success)
                    .with_key_value("ARGS", "2", CanvasTone::Text),
            );

        let draw = CanvasLayout::default().layout_document(&document, &CanvasTheme::default());

        assert!(!draw.rects.is_empty());
        assert!(!draw.text.is_empty());
        for rect in draw
            .rects
            .iter()
            .map(|rect| rect.rect)
            .chain(draw.text.iter().map(|text| text.rect))
        {
            assert!(rect[0] >= 0.0);
            assert!(rect[1] >= 0.0);
            assert!(rect[0] + rect[2] <= 1.001);
            assert!(rect[1] + rect[3] <= 1.001);
        }
    }

    #[test]
    fn wraps_long_key_values() {
        let document =
            CanvasDocument::new("WRAP").with_section(CanvasSection::new("DATA").with_key_value(
                "ADDRESS",
                "/a/very/long/address/that/should/wrap/across/multiple/canvas/rows",
                CanvasTone::Text,
            ));

        let draw = CanvasLayout {
            columns: 28,
            rows: 12,
            padding_columns: 1,
            header_rows: 2,
            key_columns: 8,
        }
        .layout_document(&document, &CanvasTheme::default());

        let value_rows = draw
            .text
            .iter()
            .filter(|run| run.role == CanvasTextRole::Body)
            .count();
        assert!(value_rows >= 2);
    }

    #[test]
    fn flags_overflow_in_small_layouts() {
        let mut section = CanvasSection::new("MANY");
        for index in 0..12 {
            section = section.with_key_value(format!("K{index}"), "VALUE", CanvasTone::Text);
        }
        let draw = CanvasLayout {
            columns: 24,
            rows: 8,
            padding_columns: 1,
            header_rows: 2,
            key_columns: 6,
        }
        .layout_document(
            &CanvasDocument::new("OVERFLOW").with_section(section),
            &CanvasTheme::default(),
        );

        assert!(draw.overflowed);
        assert!(draw.text.iter().any(|run| run.text == "..."));
    }

    #[test]
    fn diagnostic_hud_toggles_from_any_input_source() {
        let mut state = DiagnosticHudState::new(true);

        let update = state.apply(
            DiagnosticHudCommand::Toggle,
            DiagnosticHudInputSource::AdbIntent,
        );

        assert!(update.changed);
        assert!(!update.visible);
        assert_eq!(
            update.last_input_source,
            Some(DiagnosticHudInputSource::AdbIntent)
        );
        assert_eq!(update.revision, 1);

        let update = state.apply(
            DiagnosticHudCommand::Show,
            DiagnosticHudInputSource::Controller,
        );

        assert!(update.visible);
        assert_eq!(
            update.last_input_source,
            Some(DiagnosticHudInputSource::Controller)
        );
    }

    #[test]
    fn diagnostic_hud_pages_wrap_and_clamp() {
        let mut state = DiagnosticHudState::new(true).with_page_count(3);

        assert_eq!(
            state
                .apply(
                    DiagnosticHudCommand::PreviousPage,
                    DiagnosticHudInputSource::Lsl
                )
                .page_index,
            2
        );
        assert_eq!(
            state
                .apply(
                    DiagnosticHudCommand::NextPage,
                    DiagnosticHudInputSource::Osc
                )
                .page_index,
            0
        );
        assert_eq!(
            state
                .apply(
                    DiagnosticHudCommand::SetPage(99),
                    DiagnosticHudInputSource::Application
                )
                .page_index,
            2
        );

        let update = state.set_page_count(1);
        assert_eq!(update.page_index, 0);
        assert_eq!(update.page_count, 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn document_round_trips_with_serde() {
        let document = CanvasDocument::new("SERDE").with_section(
            CanvasSection::unnamed().with_badges(vec![CanvasBadge::new(
                "STATUS",
                "OK",
                CanvasTone::Success,
            )]),
        );

        let json = serde_json::to_string(&document).expect("serialize");
        let decoded: CanvasDocument = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded, document);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn diagnostic_hud_state_round_trips_with_serde() {
        let mut state = DiagnosticHudState::new(false).with_page_count(2);
        state.apply(
            DiagnosticHudCommand::SetPage(1),
            DiagnosticHudInputSource::RuntimeConfig,
        );

        let json = serde_json::to_string(&state).expect("serialize");
        let decoded: DiagnosticHudState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded, state);
        assert_eq!(decoded.page_index(), 1);
        assert_eq!(
            decoded.last_input_source(),
            Some(DiagnosticHudInputSource::RuntimeConfig)
        );
    }
}
