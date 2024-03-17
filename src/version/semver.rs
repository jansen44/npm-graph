use std::fmt::Display;

use super::token::{tokenize, Token};
use super::ParseError;

#[derive(Debug, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Vec<String>,
    pub metadata: Vec<String>,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}{}{}",
            self.major,
            self.minor,
            self.patch,
            if self.pre_release.len() > 0 {
                format!("-{}", self.pre_release.join("."))
            } else {
                "".to_owned()
            },
            if self.metadata.len() > 0 {
                format!("-{}", self.metadata.join("."))
            } else {
                "".to_owned()
            },
        )
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

#[derive(Default)]
struct VersionBuilder {
    major: Option<u32>,
    minor: Option<u32>,
    patch: Option<u32>,
    pre_release: Vec<String>,
    metadata: Vec<String>,
}

pub fn build_from_tokens(tokens: &[Token]) -> Result<Version, ParseError> {
    if tokens.len() == 0 {
        return Err(ParseError::EmptyTokenList);
    }

    let mut version = VersionBuilder::default();
    let empty_token = Token::Empty;
    let mut state = ParsingState::Core;
    let mut prev = &empty_token;
    let mut hyphen_accumulator = String::new();

    for (i, curr) in tokens.iter().enumerate() {
        let mut change_to = None;

        match state {
            ParsingState::Core => match curr {
                Token::Dot => match prev {
                    Token::Number(_) if version.patch.is_none() => (),
                    _ => return Err(ParseError::InvalidTokenAt(i)),
                },

                Token::Number(n) => match prev {
                    Token::Empty => version.major = Some(*n),
                    Token::Dot if version.minor.is_none() => version.minor = Some(*n),
                    Token::Dot if version.patch.is_none() => version.patch = Some(*n),
                    _ => return Err(ParseError::InvalidTokenAt(i)),
                },

                Token::Hyphen if *prev != Token::Dot => change_to = Some(ParsingState::PreRelease),
                Token::Plus if *prev != Token::Dot => change_to = Some(ParsingState::Metadata),

                _ => return Err(ParseError::InvalidTokenAt(i)),
            },
            ParsingState::PreRelease => match curr {
                Token::Dot => match prev {
                    Token::AlphaNumeric(_) | Token::Number(_) => (),
                    Token::Hyphen => {
                        version.pre_release.push(hyphen_accumulator.clone());
                        hyphen_accumulator = String::new();
                    }
                    _ => return Err(ParseError::InvalidTokenAt(i)),
                },
                Token::Hyphen if *prev == Token::Dot || *prev == Token::Hyphen => {
                    hyphen_accumulator.push('-');
                }
                Token::AlphaNumeric(identifier)
                    if *prev == Token::Empty || *prev == Token::Dot || *prev == Token::Hyphen =>
                {
                    version.pre_release.push(format!(
                        "{}{}",
                        hyphen_accumulator,
                        identifier.clone()
                    ));
                    hyphen_accumulator = String::new();
                }
                Token::Number(number)
                    if *prev == Token::Empty || *prev == Token::Dot || *prev == Token::Hyphen =>
                {
                    version
                        .pre_release
                        .push(format!("{}{}", hyphen_accumulator, number,));
                    hyphen_accumulator = String::new();
                }

                Token::Plus if *prev != Token::Dot => change_to = Some(ParsingState::Metadata),
                _ => return Err(ParseError::InvalidTokenAt(i)),
            },
            ParsingState::Metadata => match curr {
                Token::Dot => match prev {
                    Token::AlphaNumeric(_) | Token::Number(_) => (),
                    Token::Hyphen => {
                        version.metadata.push(hyphen_accumulator.clone());
                        hyphen_accumulator = String::new();
                    }
                    _ => return Err(ParseError::InvalidTokenAt(i)),
                },
                Token::Hyphen if *prev == Token::Dot || *prev == Token::Hyphen => {
                    hyphen_accumulator.push('-');
                }
                Token::AlphaNumeric(identifier)
                    if *prev == Token::Empty || *prev == Token::Dot || *prev == Token::Hyphen =>
                {
                    version
                        .metadata
                        .push(format!("{}{}", hyphen_accumulator, identifier.clone()));
                    hyphen_accumulator = String::new();
                }
                Token::Number(number)
                    if *prev == Token::Empty || *prev == Token::Dot || *prev == Token::Hyphen =>
                {
                    version
                        .metadata
                        .push(format!("{}{}", hyphen_accumulator, number,));
                    hyphen_accumulator = String::new();
                }

                _ => return Err(ParseError::InvalidTokenAt(i)),
            },
        }

        if let Some(change_to) = change_to {
            prev = &empty_token;
            state = change_to;
        } else {
            prev = curr;
        }
    }

    let major = version.major.ok_or(ParseError::MissingSymbolAt(0))?;
    let minor = version.minor.unwrap_or_default();
    let patch = version.patch.unwrap_or_default();

    Ok(Version {
        major,
        minor,
        patch,
        pre_release: version.pre_release,
        metadata: version.metadata,
    })
}

enum ParsingState {
    Core,
    PreRelease,
    Metadata,
}

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

        let v = "=2.1.1";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 1);

        let v = "v5.1.123";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 5);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 123);

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

    #[test]
    fn pre_release() {
        let v = "1.0.0-alpha";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 1);
        assert_eq!(version.pre_release[0], "alpha".to_owned());

        let v = "1.50-alpha.beta";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 50);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 2);
        assert_eq!(version.pre_release[0], "alpha".to_owned());
        assert_eq!(version.pre_release[1], "beta".to_owned());

        let v = "50-alpha.beta.--.omega.123.th3t4";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 50);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 6);
        assert_eq!(version.pre_release[0], "alpha".to_owned());
        assert_eq!(version.pre_release[1], "beta".to_owned());
        assert_eq!(version.pre_release[2], "--".to_owned());
        assert_eq!(version.pre_release[3], "omega".to_owned());
        assert_eq!(version.pre_release[4], "123".to_owned());
        assert_eq!(version.pre_release[5], "th3t4".to_owned());

        let v = "50-.beta.--.omega.123.th3t4";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(2));

        let v = "1.0.0-rc..1";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(8));
    }

    #[test]
    fn metadata() {
        let v = "1.0.0-alpha+test.meta";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 1);
        assert_eq!(version.pre_release[0], "alpha".to_owned());
        assert_eq!(version.metadata.len(), 2);
        assert_eq!(version.metadata[0], "test".to_owned());
        assert_eq!(version.metadata[1], "meta".to_owned());

        let v = "1.50-alpha.beta+123.321.23";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 50);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 2);
        assert_eq!(version.pre_release[0], "alpha".to_owned());
        assert_eq!(version.pre_release[1], "beta".to_owned());
        assert_eq!(version.metadata.len(), 3);
        assert_eq!(version.metadata[0], "123".to_owned());
        assert_eq!(version.metadata[1], "321".to_owned());
        assert_eq!(version.metadata[2], "23".to_owned());

        let v = "50+alpha.beta.--.omega.123.th3t4";
        let version = Version::parse(v).unwrap();
        assert_eq!(version.major, 50);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.pre_release.len(), 0);
        assert_eq!(version.metadata.len(), 6);
        assert_eq!(version.metadata[0], "alpha".to_owned());
        assert_eq!(version.metadata[1], "beta".to_owned());
        assert_eq!(version.metadata[2], "--".to_owned());
        assert_eq!(version.metadata[3], "omega".to_owned());
        assert_eq!(version.metadata[4], "123".to_owned());
        assert_eq!(version.metadata[5], "th3t4".to_owned());

        let v = "50+.beta.--.omega.123.th3t4";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(2));

        let v = "1.0.0+rc..1";
        let version = Version::parse(v).unwrap_err();
        assert_eq!(version, ParseError::InvalidTokenAt(8));
    }
}
