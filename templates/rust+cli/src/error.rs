#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("experimental command: {0}")]
    #[allow(unused)]
    ExperimentalCommand(String),
    #[error("argument {0}")]
    Argument(String),
    #[error("unknown command")]
    UnknownCommand,
}
