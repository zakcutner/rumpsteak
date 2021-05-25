use super::Fsm;
use std::fmt::{self, Display, Formatter};

#[cfg(feature = "parsing")]
pub use self::parse::{parse, ParseError};

pub struct Dot<'a, R, L>(&'a Fsm<R, L>);

impl<'a, R: Display, L: Display> Dot<'a, R, L> {
    pub fn new(fsm: &'a Fsm<R, L>) -> Self {
        Self(fsm)
    }
}

impl<'a, R: Display, L: Display> Display for Dot<'a, R, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "digraph \"{}\" {{", self.0.role())?;
        let (states, transitions) = self.0.size();

        if states > 0 {
            writeln!(f)?;
        }

        for i in self.0.states() {
            writeln!(f, "    {};", i.index())?;
        }

        if transitions > 0 {
            writeln!(f)?;
        }

        for (from, to, transition) in self.0.transitions() {
            let (from, to) = (from.index(), to.index());
            writeln!(f, "    {} -> {} [label=\"{}\"];", from, to, transition)?;
        }

        write!(f, "}}")
    }
}

#[cfg(feature = "parsing")]
mod parse {
    use crate::fsm::{Action, Fsm, Transition, TransitionError};
    use enum_kinds::EnumKind;
    use logos::{Lexer, Logos};
    use memchr::{memchr2_iter, memchr_iter};
    use std::{
        borrow::Cow,
        collections::{hash_map::Entry, HashMap, HashSet},
        fmt::{self, Display, Formatter},
    };
    use thiserror::Error;

    #[derive(Debug, EnumKind, PartialEq, Eq, Hash, Logos)]
    #[enum_kind(TokenKind)]
    enum Token<'a> {
        #[regex(r#"[a-zA-Z\x80-\xFF_][a-zA-Z\x80-\xFF_0-9]*"#, identity)]
        #[regex(r"-?(\.[0-9]+|[0-9]+(\.[0-9]*)?)", identity)]
        #[regex(r#""[^"\\]*(\\"?[^"\\]*)*""#, literal)]
        Identifier(Cow<'a, str>),

        #[token("digraph")]
        Digraph,

        #[regex(r#"label|"label""#)]
        Label,

        #[token("{")]
        LeftBrace,

        #[token("}")]
        RightBrace,

        #[token("[")]
        LeftSquare,

        #[token("]")]
        RightSquare,

        #[token("->")]
        Arrow,

        #[token("=")]
        Equals,

        #[token(",")]
        Comma,

        #[token(";")]
        Semicolon,

        #[allow(dead_code)]
        Transition,

        #[allow(dead_code)]
        EndOfInput,

        #[error]
        #[regex(r"[ \t\r\n\f\v]", logos::skip)]
        Error,
    }

    impl<'a> PartialEq<TokenKind> for Token<'a> {
        fn eq(&self, other: &TokenKind) -> bool {
            TokenKind::from(self).eq(other)
        }
    }

    impl Display for TokenKind {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                Self::Identifier => write!(f, "an identifier"),
                Self::Digraph => write!(f, "'digraph'"),
                Self::Label => write!(f, "'label'"),
                Self::LeftBrace => write!(f, "'{{'"),
                Self::RightBrace => write!(f, "'}}'"),
                Self::LeftSquare => write!(f, "'['"),
                Self::RightSquare => write!(f, "']'"),
                Self::Arrow => write!(f, "'->'"),
                Self::Equals => write!(f, "'='"),
                Self::Comma => write!(f, "','"),
                Self::Semicolon => write!(f, "';'"),
                Self::Transition => write!(f, "a transition"),
                Self::EndOfInput => write!(f, "end of input"),
                Self::Error => unreachable!(),
            }
        }
    }

    struct PeekableLexer<'a> {
        tokens: Lexer<'a, Token<'a>>,
        peeked: Option<Option<Token<'a>>>,
    }

    impl<'a> PeekableLexer<'a> {
        fn new(tokens: Lexer<'a, Token<'a>>) -> Self {
            let peeked = None;
            Self { tokens, peeked }
        }

        fn slice(&self) -> &'a str {
            self.tokens.slice()
        }

        fn peek(&mut self) -> Option<&Token<'a>> {
            let tokens = &mut self.tokens;
            let token = self.peeked.get_or_insert_with(|| tokens.next());
            token.as_ref()
        }
    }

    impl<'a> Iterator for PeekableLexer<'a> {
        type Item = Token<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.peeked.take() {
                Some(token) => token,
                None => self.tokens.next(),
            }
        }
    }

    #[derive(Debug, Error)]
    pub struct UnexpectedTokenError {
        expected: &'static [TokenKind],
        actual: Option<String>,
    }

    impl UnexpectedTokenError {
        fn new(expected: &'static [TokenKind], actual: Option<String>) -> Self {
            assert!(!expected.is_empty());
            Self { expected, actual }
        }
    }

    impl Display for UnexpectedTokenError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "expected {}", self.expected[0])?;

            let size = self.expected.len();
            if size > 1 {
                for i in 1..size - 1 {
                    write!(f, ", {}", self.expected[i])?;
                }

                write!(f, " or {}", self.expected[size - 1])?;
            }

            write!(f, " but found ")?;
            match &self.actual {
                Some(actual) => write!(f, "'{}'", actual),
                None => write!(f, "{}", TokenKind::EndOfInput),
            }
        }
    }

    #[derive(Debug, Error)]
    pub enum ParseError {
        #[error(transparent)]
        UnexpectedToken(#[from] UnexpectedTokenError),
        #[error("state '{0}' is defined multiple times")]
        DuplicateState(String),
        #[error("the same transition is defined multiple times")]
        DuplicateTransition,
        #[error("state '{0}' has not been defined")]
        UndefinedState(String),
        #[error(transparent)]
        Transition(#[from] TransitionError),
    }

    fn identity<'a>(tokens: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
        Cow::Borrowed(tokens.slice())
    }

    fn literal<'a>(tokens: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
        let mut chars = tokens.slice().chars();
        assert_eq!(chars.next(), Some('"'));
        assert_eq!(chars.next_back(), Some('"'));

        let slice = chars.as_str();
        let mut result = String::new();
        let mut last_end = 0;

        if slice.is_empty() {
            return Cow::Borrowed(slice);
        }

        let bytes = slice.as_bytes();
        assert_ne!(bytes[0], b'"');

        for start in memchr_iter(b'"', &bytes[1..]) {
            assert_eq!(slice.as_bytes()[start], b'\\');
            result.push_str(&slice[last_end..start]);
            result.push('"');
            last_end = start + 2;
        }

        if result.is_empty() {
            return Cow::Borrowed(slice);
        }

        result.push_str(&slice[last_end..slice.len()]);
        Cow::Owned(result)
    }

    fn optional<'a>(
        tokens: &mut PeekableLexer<'a>,
        expected: &'static [TokenKind],
    ) -> Option<Token<'a>> {
        match tokens.peek() {
            Some(token) if expected.iter().any(|t| token == t) => tokens.next(),
            _ => None,
        }
    }

    fn expect<'a>(
        tokens: &mut PeekableLexer<'a>,
        expected: &'static [TokenKind],
    ) -> Result<Token<'a>, UnexpectedTokenError> {
        match tokens.next() {
            Some(token) if expected.iter().any(|&t| token == t) => Ok(token),
            Some(_) => {
                let actual = Some(tokens.slice().to_owned());
                Err(UnexpectedTokenError::new(expected, actual))
            }
            None => Err(UnexpectedTokenError::new(expected, None)),
        }
    }

    fn parse_transition<'a>(
        #[allow(clippy::ptr_arg)] input: &Cow<'a, str>,
    ) -> Option<Transition<Cow<'a, str>, Cow<'a, str>>> {
        if input.len() < 3 {
            return None;
        }

        let haystack = &input.as_bytes()[1..input.len() - 1];
        let mut iter = memchr2_iter(b'?', b'!', haystack);

        let i = match (iter.next(), iter.next()) {
            (Some(i), None) => i + 1,
            _ => return None,
        };

        let action = match input.as_bytes()[i] {
            b'?' => Action::Input,
            b'!' => Action::Output,
            _ => unreachable!(),
        };

        let (role, label) = match input {
            Cow::Borrowed(input) => (Cow::Borrowed(&input[..i]), Cow::Borrowed(&input[i + 1..])),
            Cow::Owned(input) => {
                let (role, label) = (input[..i].to_owned(), input[i + 1..].to_owned());
                (Cow::Owned(role), Cow::Owned(label))
            }
        };

        Some(Transition::new(role, action, label))
    }

    pub fn parse(input: &str) -> Result<Fsm<Cow<str>, Cow<str>>, ParseError> {
        let mut tokens = PeekableLexer::new(Token::lexer(input));
        expect(&mut tokens, &[TokenKind::Digraph])?;

        let mut fsm = match expect(&mut tokens, &[TokenKind::Identifier])? {
            Token::Identifier(role) => Fsm::new(role),
            _ => unreachable!(),
        };

        expect(&mut tokens, &[TokenKind::LeftBrace])?;

        let mut states = HashMap::new();
        let mut transitions = HashSet::new();

        loop {
            let expected = &[TokenKind::RightBrace, TokenKind::Identifier];
            let left = match expect(&mut tokens, expected)? {
                Token::RightBrace => break,
                Token::Identifier(left) => left,
                _ => unreachable!(),
            };

            if expect(&mut tokens, &[TokenKind::Semicolon, TokenKind::Arrow])?
                == TokenKind::Semicolon
            {
                match states.entry(left) {
                    Entry::Occupied(entry) => {
                        return Err(ParseError::DuplicateState(entry.key().as_ref().to_owned()));
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(fsm.add_state());
                        continue;
                    }
                }
            }

            let right = match expect(&mut tokens, &[TokenKind::Identifier])? {
                Token::Identifier(right) => right,
                _ => unreachable!(),
            };

            expect(&mut tokens, &[TokenKind::LeftSquare])?;
            expect(&mut tokens, &[TokenKind::Label])?;
            expect(&mut tokens, &[TokenKind::Equals])?;

            let transition = match expect(&mut tokens, &[TokenKind::Identifier])? {
                Token::Identifier(transition) => transition,
                _ => unreachable!(),
            };

            let transition = match parse_transition(&transition) {
                Some(transition) => transition,
                None => {
                    let actual = Some(transition.into_owned());
                    return Err(UnexpectedTokenError::new(&[TokenKind::Transition], actual).into());
                }
            };

            if !transitions.insert((left, right, transition)) {
                return Err(ParseError::DuplicateTransition);
            }

            optional(&mut tokens, &[TokenKind::Comma]);
            expect(&mut tokens, &[TokenKind::RightSquare])?;
            expect(&mut tokens, &[TokenKind::Semicolon])?;
        }

        if tokens.next().is_some() {
            let actual = Some(tokens.slice().to_owned());
            return Err(UnexpectedTokenError::new(&[TokenKind::EndOfInput], actual).into());
        }

        for (from, to, transition) in transitions {
            let (&from, &to) = match (states.get(&from), states.get(&to)) {
                (Some(from), Some(to)) => (from, to),
                (None, _) => return Err(ParseError::UndefinedState(from.into_owned())),
                (_, None) => return Err(ParseError::UndefinedState(to.into_owned())),
            };

            fsm.add_transition(from, to, transition)?;
        }

        Ok(fsm)
    }
}
