//! The replay plan: every corpus file in schedule order, every statement of
//! every psql submission parsed once, so both the snapshot pass and the
//! comparison pass work from the same view of the corpus.

use super::corpus::{FrozenSplit, ItemKind};
use super::replay::{Shape, parse, statements};

/// One corpus item, ready to run or compare.
#[derive(Clone, Debug)]
pub struct PlannedItem {
    /// The statement as psql sends it, when the submission is server SQL.
    pub sql: Option<String>,
    /// What `pgtk-sql` made of it, when it parsed.
    pub shape: Option<Shape>,
    /// Whether the frozen submissions call it server SQL at all.
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
    /// Read and parse the whole corpus. Files the schedule names but the
    /// frozen submissions do not know are left out.
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
                    .flat_map(|item| match item.kind {
                        ItemKind::Statement => statements(file.text_of(item))
                            .into_iter()
                            .map(|sql| PlannedItem {
                                shape: parse(&sql),
                                sql: Some(sql),
                                kind: item.kind,
                            })
                            .collect::<Vec<_>>(),
                        ItemKind::Other => vec![PlannedItem {
                            sql: None,
                            shape: None,
                            kind: item.kind,
                        }],
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
    let text = super::corpus::frozen_file("src/test/regress/parallel_schedule");
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
