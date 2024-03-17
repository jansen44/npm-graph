pub mod condition;
pub mod semver;

mod token;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Unexpected,
    EmptyInput,
    EmptyTokenList,
    InvalidToken(char),
    InvalidTokenAt(usize),
    MissingSymbolAt(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}
