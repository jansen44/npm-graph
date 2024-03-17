use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let input = input.trim();

        if input.len() == 0 {
            return Err(ParseError::EmptyInput);
        }

        let tokens = tokenize(input)?;
        build_from_tokens(&tokens)
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut input = input.chars().peekable();
    let mut curr = input.next();
    let mut tokens = vec![];

    while curr.is_some() {
        let c = curr.unwrap();
        match c {
            ' ' | ',' => (),

            // DIGITS =====================================================
            '0' => tokens.push(Token::Number(c.to_digit(10).unwrap())),
            n if n.is_digit(10) => {
                let mut number = String::from(c);
                while input.peek().is_some_and(|c| c.is_digit(10)) {
                    number.push(input.next().unwrap());
                }
                tokens.push(Token::Number(number.parse().unwrap()));
            }

            '.' => tokens.push(Token::Dot),

            _ => return Err(ParseError::InvalidToken(c)),
        }

        curr = input.next();
    }

    Ok(tokens)
}

#[derive(Default)]
struct VersionBuilder {
    major: Option<u32>,
    minor: Option<u32>,
    patch: Option<u32>,
}

fn build_from_tokens(tokens: &[Token]) -> Result<Version, ParseError> {
    if tokens.len() == 0 {
        return Err(ParseError::EmptyTokenList);
    }

    let mut version = VersionBuilder::default();

    for (i, curr) in tokens.iter().enumerate() {
        match curr {
            Token::Dot => match i {
                1 | 3 => (),
                _ => return Err(ParseError::InvalidTokenAt(i)),
            },
            Token::Number(n) => match i {
                0 => version.major = Some(*n),
                2 => version.minor = Some(*n),
                4 => version.patch = Some(*n),
                _ => return Err(ParseError::InvalidTokenAt(i)),
            },
        }
    }

    let major = version.major.ok_or(ParseError::MissingSymbolAt(0))?;
    let minor = version.minor.unwrap_or_default();
    let patch = version.patch.unwrap_or_default();

    Ok(Version {
        major,
        minor,
        patch,
    })
}

enum Token {
    Dot,
    Number(u32),
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    EmptyInput,
    EmptyTokenList,
    InvalidToken(char),
    InvalidTokenAt(usize),
    MissingSymbolAt(usize),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_case() {
        let v = "1.0.0";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);

        let v = "20";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 20);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);

        let v = "23.32";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 23);
        assert_eq!(version.minor, 32);
        assert_eq!(version.patch, 0);

        let v = "1.2.196";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 196);

        let v = ".1.1";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(0));

        let v = "..1";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(0));

        let v = "1..";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(2));

        let v = "1.0.0.";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(5));

        let v = "1.0.0.12";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(5));
    }
}
