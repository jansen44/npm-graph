use super::{
    semver::Version,
    token::{self, tokenize, Token},
    ParseError,
};

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(Condition::Any, _cond));

        let cond = "=2.3.4";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
            Condition::Simple(Version {
                major: 2,
                minor: 3,
                patch: 4,
                metadata: vec![],
                pre_release: vec![]
            }),
            _cond
        ));

        let cond = "~2.3";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
            Condition::Compatible(Version {
                major: 2,
                minor: 3,
                patch: 0,
                metadata: vec![],
                pre_release: vec![]
            }),
            _cond
        ));

        let cond = "^52.13.194";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
            Condition::CompatibleWithMostRecent(Version {
                major: 52,
                minor: 13,
                patch: 194,
                metadata: vec![],
                pre_release: vec![]
            }),
            _cond
        ));
    }

    #[test]
    fn range_cases() {
        let cond = ">1.2.3";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
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
            _cond
        ));

        let cond = ">=4.15.3-beta.1";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
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
            _cond
        ));

        let cond = ">1.2.3 <4.15.3-beta.1";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
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
            _cond
        ));

        let cond = ">=1.2.3 <4.15.3";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
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
            _cond
        ));

        let cond = ">=1.2.3 <=4.15.3";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
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
            _cond
        ));
    }

    #[test]
    fn composite_cases() {
        let cond = ">=1.2.3 <=4.15.3 || 5";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
            Condition::Composite(vec![
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
                Condition::Simple(Version {
                    major: 5,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                })
            ]),
            _cond
        ));

        let cond = "1 || 2 || 3 || 4 || ^5";
        let _cond = Condition::parse(cond).unwrap();
        assert!(matches!(
            Condition::Composite(vec![
                Condition::Simple(Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Condition::Simple(Version {
                    major: 2,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Condition::Simple(Version {
                    major: 3,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Condition::Simple(Version {
                    major: 4,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Condition::Simple(Version {
                    major: 5,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
                Condition::CompatibleWithMostRecent(Version {
                    major: 5,
                    minor: 0,
                    patch: 0,
                    metadata: vec![],
                    pre_release: vec![]
                }),
            ]),
            _cond
        ));
    }
}
