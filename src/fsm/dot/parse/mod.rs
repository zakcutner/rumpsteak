#![cfg(feature = "parsing")]

pub mod fsm;
pub mod transition;

use self::fsm::FsmError;
use crate::fsm::{Expression, Fsm};
use logos::{Logos, Span};
use std::{
    convert::Infallible,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
    result::Result,
};
use thiserror::Error;

trait Token<'a>: Logos<'a> {
    const EOI: Self;

    type Id: TokenId;

    fn id(&self) -> Self::Id {
        unsafe { *(self as *const _ as *const _) }
    }
}

pub trait TokenId: Copy + Debug + Eq + 'static {
    fn name(self) -> &'static str;
}

trait Merge {
    fn merge(&self, other: &Self) -> Self;
}

impl Merge for Span {
    fn merge(&self, other: &Self) -> Self {
        self.start..other.end
    }
}

#[derive(Clone, Debug, Eq)]
struct Spanned<T> {
    span: Span,
    inner: T,
}

impl<T> Spanned<T> {
    fn new(span: Span, inner: T) -> Self {
        Self { span, inner }
    }

    fn as_parts(&self) -> (&Span, &T) {
        (&self.span, &self.inner)
    }

    fn into_parts(self) -> (Span, T) {
        (self.span, self.inner)
    }

    fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            span: self.span,
            inner: f(self.inner),
        }
    }
}

impl<'a, T: Token<'a>> Spanned<T> {
    fn from_tokens(tokens: &mut logos::Lexer<'a, T>) -> Self {
        Self::new(
            tokens.span(),
            match tokens.next() {
                Some(token) => token,
                None => T::EOI,
            },
        )
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Hash> Hash for Spanned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

struct Lexer<'a, T: Token<'a>, E> {
    tokens: logos::Lexer<'a, T>,
    peeked: Option<Spanned<T>>,
    expected: Vec<&'static str>,
    errors: E,
}

impl<'a, T: Token<'a>, E> Lexer<'a, T, E> {
    fn new(tokens: logos::Lexer<'a, T>, errors: E) -> Self {
        Self {
            tokens,
            peeked: Default::default(),
            expected: Default::default(),
            errors,
        }
    }

    fn finish(&self) {
        assert!(self.expected.is_empty());
    }

    fn take_errors(&mut self) -> E
    where
        E: Default,
    {
        mem::take(&mut self.errors)
    }

    fn peek(&mut self) -> &Spanned<T> {
        let tokens = &mut self.tokens;
        self.peeked
            .get_or_insert_with(|| Spanned::from_tokens(tokens))
    }

    fn next(&mut self) -> Spanned<T> {
        match self.peeked.take() {
            Some(token) => token,
            None => Spanned::from_tokens(&mut self.tokens),
        }
    }

    fn next_if(&mut self, id: T::Id) -> Option<Spanned<T>> {
        if self.peek().inner.id() == id {
            self.expected.clear();
            return Some(self.next());
        }

        self.expected.push(id.name());
        None
    }
}

impl<'a, T: Token<'a>, E: PushError> PushError for Lexer<'a, T, E> {
    fn push_err(&mut self, span: Span, err: ParseError) {
        self.errors.push_err(span, err)
    }
}

impl<'a, T: Token<'a>, E: PushError> Lexer<'a, T, E> {
    fn expect(&mut self) {
        assert!(!self.expected.is_empty());
        let span = self.peek().span.clone();
        let expected = mem::take(&mut self.expected);
        self.push_err(span, TokenError(expected).into());
    }

    fn expect_next_if(&mut self, id: T::Id) -> Option<Spanned<T>> {
        let output = self.next_if(id);
        if output.is_none() {
            self.expect();
        }

        output
    }
}

#[derive(Debug, Error)]
pub struct TokenError(Vec<&'static str>);

impl Display for TokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "expected {}", self.0[0])?;

        let size = self.0.len();
        if size > 1 {
            for i in 1..size - 1 {
                write!(f, ", {}", self.0[i])?;
            }

            write!(f, " or {}", self.0[size - 1])?;
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
enum ParseError {
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error(transparent)]
    Fsm(#[from] FsmError),
}

#[derive(Debug, Default, Error)]
pub struct ParseErrors {
    items: Vec<Spanned<ParseError>>,
}

impl ParseErrors {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Display for ParseErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (span, err) in self.items.iter().map(Spanned::as_parts) {
            writeln!(f, "error at {}: {}", span.start, err)?;
        }

        Ok(())
    }
}

trait PushError {
    fn push_err(&mut self, span: Span, err: ParseError);
}

impl<E: PushError> PushError for &mut E {
    fn push_err(&mut self, span: Span, err: ParseError) {
        (*self).push_err(span, err);
    }
}

impl PushError for &mut dyn PushError {
    fn push_err(&mut self, span: Span, err: ParseError) {
        (*self).push_err(span, err);
    }
}

impl PushError for ParseErrors {
    fn push_err(&mut self, span: Span, err: ParseError) {
        self.items.push(Spanned::new(span, err));
    }
}

struct ParseIter<'a, E: transition::Expression> {
    tokens: Lexer<'a, fsm::Token<'a>, ParseErrors>,
    phantom: PhantomData<E>,
}

impl<'a, E: transition::Expression> Iterator for ParseIter<'a, E> {
    type Item = Result<Fsm<String, String, E>, ParseErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.tokens.peek().inner {
            fsm::Token::Eoi => None,
            _ => {
                let fsm = fsm::parse(&mut self.tokens);
                self.tokens.finish();
                let errors = self.tokens.take_errors();
                if !errors.is_empty() {
                    return Some(Err(errors));
                }

                Some(Ok(fsm.unwrap()))
            }
        }
    }
}

pub fn parse(
    source: &str,
) -> impl Iterator<Item = Result<Fsm<String, String, Infallible>, ParseErrors>> + '_ {
    ParseIter {
        tokens: Lexer::new(fsm::Token::lexer(source), Default::default()),
        phantom: PhantomData,
    }
}

pub fn parse_with_refinements(
    source: &str,
) -> impl Iterator<Item = Result<Fsm<String, String, Expression<String>>, ParseErrors>> + '_ {
    ParseIter {
        tokens: Lexer::new(fsm::Token::lexer(source), Default::default()),
        phantom: PhantomData,
    }
}
