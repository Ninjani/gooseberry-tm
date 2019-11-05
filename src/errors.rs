use thiserror::Error;

use crate::entry::GooseberryEntryType;

#[derive(Debug, Error)]
pub enum Sorry {
    #[error(
        "What's {entry_type:?}? I can only remember Tasks, Research, Events and Journal entries."
    )]
    UnknownEntryType { entry_type: String },
    #[error("Entry {entry_type:?}_{entry_id:?} hasn't been written yet")]
    MissingEntryID {
        entry_type: GooseberryEntryType,
        entry_id: u64,
    },
    #[error("Every entry needs a header section (demarcated by ---) so I know what it's about")]
    MissingHeader,
    #[error("An entry of this type needs the '{element:?}' element in its header")]
    MissingHeaderElement { element: String },
    #[error("Expected {expected:?}, got {got:?}")]
    WrongEntryType {
        expected: GooseberryEntryType,
        got: GooseberryEntryType,
    },
    #[error("Redo from start. {message:?}")]
    OutOfCheeseError { message: String },
}
