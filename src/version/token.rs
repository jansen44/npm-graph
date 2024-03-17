use super::ParseError;

#[derive(Debug, PartialEq)]
pub enum Token {
    Empty,

    Asterisk,
    Dot,
    Hyphen,
    Plus,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Caret,
    Tilde,
    Or,
    Number(u32),
    AlphaNumeric(String),
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut input = input.chars().peekable();
    let mut curr = input.next();
    let mut tokens = vec![];

    if let Some(c) = curr {
        if c == '=' {
            curr = input.next();
        } else if c == 'v' {
            curr = input.next();
        }
    }

    while curr.is_some() {
        let c = curr.unwrap();
        match c {
            ' ' | ',' => (),

            '*' => tokens.push(Token::Asterisk),
            '.' => tokens.push(Token::Dot),
            '-' => tokens.push(Token::Hyphen),
            '+' => tokens.push(Token::Plus),

            '0' => tokens.push(Token::Number(c.to_digit(10).unwrap())),

            '~' => tokens.push(Token::Tilde),
            '^' => tokens.push(Token::Caret),
            '>' => {
                if input.peek().is_some_and(|c| *c == '=') {
                    input.next();
                    tokens.push(Token::GreaterEqual);
                } else {
                    tokens.push(Token::Greater);
                }
            }
            '<' => {
                if input.peek().is_some_and(|c| *c == '=') {
                    input.next();
                    tokens.push(Token::LessEqual);
                } else {
                    tokens.push(Token::Less);
                }
            }
            '|' if input.peek().is_some_and(|c| *c == '|') => {
                input.next();
                tokens.push(Token::Or);
            }

            n if n.is_alphanumeric() || n == '-' => {
                let mut current_token = String::from(c);
                while input
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || n == '-')
                {
                    current_token.push(input.next().unwrap());
                }

                let token = match current_token.parse::<u32>() {
                    Ok(number) => Token::Number(number),
                    Err(_) => Token::AlphaNumeric(current_token),
                };
                tokens.push(token);
            }

            _ => return Err(ParseError::InvalidToken(c)),
        }

        curr = input.next();
    }

    Ok(tokens)
}
