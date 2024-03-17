use super::{
    semver::Version,
    token::{tokenize, Token},
    ParseError,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ConditionRange {
    Less(Version),
    LessEqual(Version),
    Greater(Version),
    GreaterEqual(Version),
}

impl std::fmt::Display for ConditionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let gap = match self {
            ConditionRange::Less(version) => format!("<{version}"),
            ConditionRange::LessEqual(version) => format!("<={version}"),
            ConditionRange::Greater(version) => format!(">{version}"),
            ConditionRange::GreaterEqual(version) => format!(">={version}"),
        };
        write!(f, "{}", gap)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    Any,
    Simple(Version),
    Compatible(Version),
    CompatibleWithMostRecent(Version),
    Range(ConditionRange, Option<ConditionRange>),
    Composite(Vec<Condition>),
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let condition = match self {
            Condition::Any => "*".to_owned(),
            Condition::Simple(version) => version.to_string(),
            Condition::Compatible(version) => format!("~{version}").to_string(),
            Condition::CompatibleWithMostRecent(version) => format!("^{version}").to_string(),
            Condition::Range(v1, v2) => format!(
                "{v1}{}",
                if v2.is_some() {
                    format!(" {}", v2.clone().unwrap().to_string())
                } else {
                    "".to_owned()
                }
            ),
            Condition::Composite(versions) => versions
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(" || "),
        };

        write!(f, "{}", condition)
    }
}

impl Condition {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let input = input.trim();

        if input.len() == 0 {
            return Err(ParseError::EmptyInput);
        }

        let tokens = tokenize(input)?;
        build_from_tokens(&tokens)
    }

    pub fn compare(&self, version: &Version) -> bool {
        match self {
            Condition::Any => true,
            Condition::Simple(v) => v == version,
            Condition::Compatible(v) => {
                v.major == version.major && v.minor == version.minor && v.patch <= version.patch
            }
            Condition::CompatibleWithMostRecent(v) => {
                !(v.major != version.major
                    || v.minor > version.minor
                    || (v.minor == version.minor && v.patch > version.patch))
            }

            Condition::Range(left, right) => {
                let version = version.get_version_offset();

                let left_offset = match left {
                    ConditionRange::Greater(v) => v.get_version_offset() < version,
                    ConditionRange::GreaterEqual(v) => v.get_version_offset() <= version,
                    _ => unreachable!("NEVER BEGINS WITH LESS"),
                };

                if !left_offset {
                    return false;
                }

                match right {
                    Some(right) => match right {
                        ConditionRange::Less(v) => v.get_version_offset() > version,
                        ConditionRange::LessEqual(v) => v.get_version_offset() >= version,
                        _ => unreachable!("NEVER BEGINS WITH LESS"),
                    },
                    None => true,
                }
            }
            Condition::Composite(conditions) => {
                conditions.iter().find(|c| c.compare(version)).is_some()
            }
        }
    }
}

fn build_from_tokens(tokens: &[Token]) -> Result<Condition, ParseError> {
    if tokens.len() == 0 {
        return Err(ParseError::EmptyTokenList);
    }

    if tokens.contains(&Token::Or) {
        let mut tokens = tokens;

        let mut idx = tokens.iter().position(|t| t == &Token::Or);
        let mut conditions = vec![];
        while idx.is_some() {
            let condition = build_from_tokens(&tokens[..idx.unwrap()])?;
            conditions.push(condition);

            tokens = &tokens[idx.unwrap() + 1..];
            idx = tokens.iter().position(|t| t == &Token::Or);
        }

        let condition = build_from_tokens(tokens)?;
        conditions.push(condition);

        return Ok(Condition::Composite(conditions));
    }

    match &tokens[0] {
        Token::Asterisk => Ok(Condition::Any),
        Token::Caret => {
            let version = super::semver::build_from_tokens(&tokens[1..])?;
            Ok(Condition::CompatibleWithMostRecent(version))
        }
        Token::Tilde => {
            let version = super::semver::build_from_tokens(&tokens[1..])?;
            Ok(Condition::Compatible(version))
        }
        Token::Greater => build_range_condition_from_tokens(tokens, Token::Greater),
        Token::GreaterEqual => build_range_condition_from_tokens(tokens, Token::GreaterEqual),
        _ => {
            let version = super::semver::build_from_tokens(tokens)?;
            Ok(Condition::Simple(version))
        }
    }
}

fn build_range_condition_from_tokens(
    tokens: &[Token],
    left_token: Token,
) -> Result<Condition, ParseError> {
    let idx = tokens
        .iter()
        .position(|t| *t == Token::Less || *t == Token::LessEqual);
    if idx.is_none() {
        let v = super::semver::build_from_tokens(&tokens[1..])?;
        return Ok(Condition::Range(
            range_condition_from_token(left_token, v),
            None,
        ));
    }

    let idx = idx.unwrap();
    let v1 = super::semver::build_from_tokens(&tokens[1..idx])?;
    let left_condition = range_condition_from_token(left_token, v1);

    let tokens = &tokens[idx..];
    match &tokens[0] {
        Token::Less => {
            let v2 = super::semver::build_from_tokens(&tokens[1..])?;
            Ok(Condition::Range(
                left_condition,
                Some(ConditionRange::Less(v2)),
            ))
        }
        Token::LessEqual => {
            let v2 = super::semver::build_from_tokens(&tokens[1..])?;
            Ok(Condition::Range(
                left_condition,
                Some(ConditionRange::LessEqual(v2)),
            ))
        }
        _ => return Err(ParseError::Unexpected),
    }
}

fn range_condition_from_token(token: Token, version: Version) -> ConditionRange {
    match token {
        Token::Greater => ConditionRange::Greater(version),
        Token::GreaterEqual => ConditionRange::GreaterEqual(version),
        _ => unreachable!("Should never get here!"),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_cases() {
        let cond = "*";
        let cond = Condition::parse(cond).unwrap();
        assert!(matches!(cond, Condition::Any));

        let cond = "=2.3.4";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Simple(Version {
                major: 2,
                minor: 3,
                patch: 4,
                ..Default::default()
            }),
        );

        let cond = "~2.3";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Compatible(Version {
                major: 2,
                minor: 3,
                ..Default::default()
            }),
        );

        let cond = "^52.13.194";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::CompatibleWithMostRecent(Version {
                major: 52,
                minor: 13,
                patch: 194,
                ..Default::default()
            }),
        );
    }

    #[test]
    fn range_cases() {
        let cond = ">1.2.3";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Range(
                ConditionRange::Greater(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                None
            ),
        );

        let cond = ">=4.15.3-beta.1";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Range(
                ConditionRange::GreaterEqual(Version {
                    major: 4,
                    minor: 15,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec!["beta".to_string(), "1".to_string()]
                }),
                None
            ),
        );

        let cond = ">1.2.3 <4.15.3-beta.1";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Range(
                ConditionRange::Greater(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Some(ConditionRange::Less(Version {
                    major: 4,
                    minor: 15,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec!["beta".to_string(), "1".to_string()]
                })),
            ),
        );

        let cond = ">=1.2.3 <4.15.3";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Range(
                ConditionRange::GreaterEqual(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Some(ConditionRange::Less(Version {
                    major: 4,
                    minor: 15,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                })),
            ),
        );

        let cond = ">=1.2.3 <=4.15.3";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Range(
                ConditionRange::GreaterEqual(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Some(ConditionRange::LessEqual(Version {
                    major: 4,
                    minor: 15,
                    patch: 3,
                    metadata: vec![],
                    pre_release: vec![]
                })),
            ),
        );
    }

    #[test]
    fn composite_cases() {
        let cond = ">=1.2.3 <=4.15.3 || 5";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Composite(vec![
                Condition::Range(
                    ConditionRange::GreaterEqual(Version {
                        major: 1,
                        minor: 2,
                        patch: 3,
                        ..Default::default()
                    }),
                    Some(ConditionRange::LessEqual(Version {
                        major: 4,
                        minor: 15,
                        patch: 3,
                        ..Default::default()
                    })),
                ),
                Condition::Simple(Version {
                    major: 5,
                    ..Default::default()
                })
            ])
        );

        let cond = "1 || 2 || 3 || 4 || ^5";
        let cond = Condition::parse(cond).unwrap();
        assert_eq!(
            cond,
            Condition::Composite(vec![
                Condition::Simple(Version {
                    major: 1,
                    ..Default::default()
                }),
                Condition::Simple(Version {
                    major: 2,
                    ..Default::default()
                }),
                Condition::Simple(Version {
                    major: 3,
                    ..Default::default()
                }),
                Condition::Simple(Version {
                    major: 4,
                    ..Default::default()
                }),
                Condition::CompatibleWithMostRecent(Version {
                    major: 5,
                    ..Default::default()
                })
            ])
        );
    }

    #[test]
    fn compare() {
        let cond = "*";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 1,
            minor: 12,
            patch: 90,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 6,
            minor: 12,
            patch: 9,
            ..Default::default()
        }));

        let cond = "6.12.9";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 6,
            minor: 12,
            patch: 9,
            ..Default::default()
        }));

        let cond = "~5.1";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 5,
            minor: 1,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 1,
            patch: 10,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 2,
            patch: 12,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 3,
            ..Default::default()
        }));

        let cond = "^5.1";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 5,
            minor: 1,
            patch: 0,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 99,
            patch: 10,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 1,
            patch: 12,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 0,
            patch: 12,
            ..Default::default()
        }));

        let cond = ">5.2 <=8.2";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 5,
            minor: 3,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 2,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 7,
            minor: 1,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 8,
            minor: 1,
            patch: 1200,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 8,
            minor: 1,
            patch: 12,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 8,
            minor: 2,
            patch: 1,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 8,
            minor: 3,
            ..Default::default()
        }));

        let cond = ">5.2";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 9999,
            minor: 99,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 3,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 2,
            ..Default::default()
        }));

        let cond = ">=5.2";
        let cond = Condition::parse(cond).unwrap();
        assert!(cond.compare(&Version {
            major: 9999,
            minor: 99,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 3,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 2,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 1,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 4,
            minor: 1,
            ..Default::default()
        }));

        let cond = ">=5.2 <7";
        let cond = Condition::parse(cond).unwrap();
        assert!(!cond.compare(&Version {
            major: 7,
            minor: 9,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 3,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 2,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 5,
            minor: 1,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 7,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 6,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 6,
            minor: 99,
            ..Default::default()
        }));

        let cond = "1 || 2 || 3 || 4 || ^5";
        let cond = Condition::parse(cond).unwrap();
        dbg!(cond.clone());
        assert!(cond.compare(&Version {
            major: 1,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 2,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 3,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 4,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 99,
            patch: 0,
            ..Default::default()
        }));
        assert!(cond.compare(&Version {
            major: 5,
            minor: 9,
            patch: 100,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 6,
            ..Default::default()
        }));
        assert!(!cond.compare(&Version {
            major: 6,
            minor: 10,
            ..Default::default()
        }));
    }
}
