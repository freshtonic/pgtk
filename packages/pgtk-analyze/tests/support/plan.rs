//! The replay plan: every corpus file in schedule order, every item rendered
//! and parsed once, so both the snapshot pass and the comparison pass work
//! from the same view of the corpus.

use super::corpus::{FrozenSplit, ItemKind};
use super::replay::{Shape, parse, render};

/// One corpus item, ready to run or compare.
#[derive(Clone, Debug)]
pub struct PlannedItem {
    /// The item as psql would send it, when it is a statement `pgtk-psql`
    /// could render.
    pub sql: Option<String>,
    /// What `pgtk-sql` made of it, when it parsed.
    pub shape: Option<Shape>,
    /// Whether the frozen split calls it a statement at all.
    pub kind: ItemKind,
}

/// One corpus file with its position in the schedule.
#[derive(Clone, Debug)]
pub struct PlannedFile {
    pub name: String,
    /// Index of the schedule group the file belongs to.
    pub group: usize,
    /// Index of the file inside its group.
    pub index: usize,
    pub items: Vec<PlannedItem>,
}

/// The whole corpus, grouped as `parallel_schedule` groups it.
#[derive(Debug)]
pub struct Plan {
    pub groups: Vec<Vec<PlannedFile>>,
}

impl Plan {
    /// Read, render and parse the whole corpus. Files the schedule names but
    /// the frozen split does not know are left out.
    pub fn build(split: &FrozenSplit) -> Self {
        let mut groups = Vec::new();
        for names in schedule_groups() {
            let group_index = groups.len();
            let mut files = Vec::new();
            for name in names {
                if !split.file_names().any(|known| known == name) {
                    continue;
                }
                let file = split.read(&name);
                let items = file
                    .items
                    .iter()
                    .map(|item| {
                        let raw = file.text_of(item);
                        let sql = (item.kind == ItemKind::Statement)
                            .then(|| render(raw))
                            .flatten();
                        let shape = sql.as_deref().and_then(parse);
                        PlannedItem {
                            sql,
                            shape,
                            kind: item.kind,
                        }
                    })
                    .collect::<Vec<_>>();
                files.push(PlannedFile {
                    name,
                    group: group_index,
                    index: files.len(),
                    items,
                });
            }
            if !files.is_empty() {
                groups.push(files);
            }
        }
        Self { groups }
    }

    /// Every file in schedule order.
    pub fn files(&self) -> impl Iterator<Item = &PlannedFile> {
        self.groups.iter().flatten()
    }
}

/// The schedule as groups of file names.
fn schedule_groups() -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let path = super::corpus::postgres_dir().join("src/test/regress/parallel_schedule");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    for line in text.lines() {
        if let Some(names) = line.strip_prefix("test:") {
            groups.push(
                names
                    .split_whitespace()
                    .map(|name| format!("{name}.sql"))
                    .collect(),
            );
        }
    }
    groups
}
