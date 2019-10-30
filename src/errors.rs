use thiserror::Error;

#[derive(Debug, Error)]
pub enum Sorry {
    #[error(
        "What's {entry_type:?}? I can only remember Tasks, Research, Events and Journal entries."
    )]
    UnknownEntryType { entry_type: String },
    #[error("Every entry needs a header section (demarcated by ---) so I know what it's about")]
    MissingHeader,
    #[error("An entry of this type needs the '{element:?}' element in its header")]
    MissingHeaderElement { element: String },
    #[error("Your $EDITOR didn't work")]
    EditorError,
}
