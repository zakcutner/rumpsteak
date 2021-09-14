use super::{
    transition::{self, Expression, TransitionError},
    Merge, ParseErrors, PushError, Spanned,
};
use crate::{AddTransitionError, Fsm, StateIndex, Transition};
use bitvec::{bitbox, boxed::BitBox};
use logos::Logos;
use memchr::memchr_iter;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use thiserror::Error;

pub(super) type Lexer<'a> = super::Lexer<'a, Token<'a>, ParseErrors>;

type Transitions<'a, E> = HashSet<(
    Spanned<Identifier<'a>>,
    Spanned<Identifier<'a>>,
    Transition<String, String, E>,
)>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Identifier<'a> {
    Borrowed(&'a str),
    Owned(String, BitBox),
}

impl Identifier<'_> {
    fn into_string(self) -> String {
        match self {
            Self::Borrowed(identifier) => identifier.to_owned(),
            Self::Owned(identifier, _) => identifier,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Logos)]
#[repr(u8)]
pub enum Token<'a> {
    #[regex(r#"[a-zA-Z\x80-\xFF_][a-zA-Z\x80-\xFF_0-9]*"#, identity)]
    #[regex(r"-?(\.[0-9]+|[0-9]+(\.[0-9]*)?)", identity)]
    #[regex(r#""[^"\\]*(\\"?[^"\\]*)*""#, literal)]
    Identifier(Identifier<'a>),

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
    Equal,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    Eoi,

    #[error]
    #[regex(r"[ \t\r\n\f\v]", logos::skip)]
    Error,
}

impl<'a> Token<'a> {
    fn into_identifier(self) -> Identifier<'a> {
        match self {
            Self::Identifier(identifier) => identifier,
            _ => unreachable!(),
        }
    }
}

impl<'a> super::Token<'a> for Token<'a> {
    const EOI: Self = Self::Eoi;

    type Id = TokenId;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(dead_code)]
pub enum TokenId {
    Identifier,
    Digraph,
    Label,
    LeftBrace,
    RightBrace,
    LeftSquare,
    RightSquare,
    Arrow,
    Equal,
    Comma,
    Semicolon,
    Eoi,
    Error,
}

impl super::TokenId for TokenId {
    fn name(self) -> &'static str {
        match self {
            Self::Identifier => "an identifier",
            Self::Digraph => "'digraph'",
            Self::Label => "'label'",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::LeftSquare => "'['",
            Self::RightSquare => "']'",
            Self::Arrow => "'->'",
            Self::Equal => "'='",
            Self::Comma => "','",
            Self::Semicolon => "';'",
            Self::Eoi => "end of input",
            Self::Error => unreachable!(),
        }
    }
}

fn identity<'a>(tokens: &mut logos::Lexer<'a, Token<'a>>) -> Identifier<'a> {
    Identifier::Borrowed(tokens.slice())
}

fn literal<'a>(tokens: &mut logos::Lexer<'a, Token<'a>>) -> Identifier<'a> {
    let mut chars = tokens.slice().chars();
    assert_eq!(chars.next(), Some('"'));
    assert_eq!(chars.next_back(), Some('"'));

    let slice = chars.as_str();
    let mut result = String::new();
    let mut removed = bitbox![0; slice.len() - 1];
    let mut last_end = 0;

    if slice.is_empty() {
        return Identifier::Borrowed(slice);
    }

    let bytes = slice.as_bytes();
    assert_ne!(bytes[0], b'"');

    for start in memchr_iter(b'"', &bytes[1..]) {
        assert_eq!(slice.as_bytes()[start], b'\\');
        result.push_str(&slice[last_end..start]);
        result.push('"');
        removed.set(start, true);
        last_end = start + 2;
    }

    if result.is_empty() {
        return Identifier::Borrowed(slice);
    }

    result.push_str(&slice[last_end..slice.len()]);
    Identifier::Owned(result, removed)
}

#[derive(Debug, Error)]
pub enum FsmError {
    #[error("state is defined multiple times")]
    DuplicateState,
    #[error("the same transition is defined multiple times")]
    DuplicateTransition,
    #[error("state has not been defined")]
    UndefinedState,
    #[error(transparent)]
    AddTransition(#[from] AddTransitionError),
    #[error(transparent)]
    ParseTransition(#[from] TransitionError),
}

fn parse_entry<'a, E: Expression>(
    mut tokens: &mut Lexer<'a>,
    fsm: &mut Fsm<String, String, E>,
    states: &mut HashMap<Identifier<'a>, StateIndex>,
    transitions: &mut Transitions<'a, E>,
) -> Option<()> {
    if tokens.next_if(TokenId::RightBrace).is_some() {
        return None;
    }

    let left = tokens.expect_next_if(TokenId::Identifier)?;
    let left = left.map(Token::into_identifier);

    if tokens.next_if(TokenId::Semicolon).is_some() {
        match states.entry(left.inner) {
            Entry::Occupied(_) => {
                tokens.push_err(left.span, FsmError::DuplicateState.into());
            }
            Entry::Vacant(entry) => {
                entry.insert(fsm.add_state());
            }
        }
    } else {
        tokens.expect_next_if(TokenId::Arrow)?;

        let right = tokens.expect_next_if(TokenId::Identifier)?;
        let right = right.map(Token::into_identifier);

        tokens.expect_next_if(TokenId::LeftSquare)?;
        tokens.expect_next_if(TokenId::Label)?;
        tokens.expect_next_if(TokenId::Equal)?;

        let transition = tokens.expect_next_if(TokenId::Identifier)?;
        let transition = match transition.inner.into_identifier() {
            Identifier::Borrowed(source) => {
                let mut tokens =
                    transition::Lexer::new(transition::Token::lexer(source), &mut tokens);
                let transition = super::transition::parse(&mut tokens);
                tokens.finish();
                transition
            }
            Identifier::Owned(source, _) => {
                let mut tokens =
                    transition::Lexer::new(transition::Token::lexer(&source), &mut tokens);
                let transition = super::transition::parse(&mut tokens);
                tokens.finish();
                transition
            }
        };

        if let Some(transition) = transition {
            let span = left.span.merge(&right.span);
            if !transitions.insert((left, right, transition)) {
                tokens.push_err(span, FsmError::DuplicateTransition.into());
            }
        }

        tokens.next_if(TokenId::Comma);
        tokens.expect_next_if(TokenId::RightSquare)?;
        tokens.expect_next_if(TokenId::Semicolon)?;
    }

    Some(())
}

pub(super) fn parse<E: Expression>(tokens: &mut Lexer) -> Option<Fsm<String, String, E>> {
    tokens.expect_next_if(TokenId::Digraph)?;

    let role = tokens.expect_next_if(TokenId::Identifier)?;
    let mut fsm = Fsm::new(role.inner.into_identifier().into_string());

    tokens.expect_next_if(TokenId::LeftBrace)?;

    let mut states = HashMap::new();
    let mut transitions = HashSet::new();

    while parse_entry(tokens, &mut fsm, &mut states, &mut transitions).is_some() {}

    for (from, to, transition) in transitions {
        let span = from.span.merge(&to.span);

        let from = match states.get(&from.inner) {
            Some(&from) => from,
            None => {
                tokens.push_err(from.span, FsmError::UndefinedState.into());
                continue;
            }
        };

        let to = match states.get(&to.inner) {
            Some(&to) => to,
            None => {
                tokens.push_err(to.span, FsmError::UndefinedState.into());
                continue;
            }
        };

        if let Err(err) = fsm.add_transition(from, to, transition) {
            tokens.push_err(span, FsmError::from(err).into());
        }
    }

    Some(fsm)
}
