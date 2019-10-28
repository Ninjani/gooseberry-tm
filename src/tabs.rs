use anyhow::Error;
use path_abs::{PathDir, PathFile, PathOps};

use crate::entry::GooseberryEntry;

pub struct GooseberryTabs {
    task_tab: GooseberryTab,
    research_tab: GooseberryTab,
    journal_tab: GooseberryTab,
    event_tab: GooseberryTab,
}

pub struct GooseberryTab {
    ids: Vec<u64>,
    entries: Vec<GooseberryEntry>,
}

impl GooseberryTab {
    pub fn from_ids(ids: Vec<u64>, folder: PathDir) -> Result<Self, Error> {
        let filenames = ids
            .iter()
            .map(|i| PathFile::new(folder.join(format!("{}.md", i))))
            .collect::<Result<Vec<_>, _>>()?;
        let entries = filenames
            .into_iter()
            .map(|filename| GooseberryEntry::from_file(&filename))
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(GooseberryTab { ids, entries })
    }
}
