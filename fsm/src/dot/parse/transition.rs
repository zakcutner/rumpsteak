use super::{fsm::FsmError, Error, Merge, ParseError, PushError, Spanned};
use crate::{
    Action, Associativity, BinaryOp, Message, NamedParameter, Operator, Parameters, Transition,
    UnaryOp,
};
use logos::Logos;
use std::{convert::Infallible, hash::Hash};

pub(super) type Lexer<'a> = super::Lexer<'a, Token<'a>, &'a mut dyn PushError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Logos)]
pub enum Token<'a> {
    #[regex(r"[a-zA-Z\x80-\xFF_][a-zA-Z\x80-\xFF_0-9]*")]
    Identifier(&'a str),

    #[token("true", |_| true)]
    #[token("false", |_| false)]
    Boolean(bool),

    #[regex("[0-9]+", |tokens| tokens.slice().parse())]
    Number(usize),

    #[token("(")]
    LeftRound,

    #[token(")")]
    RightRound,

    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    #[token("[")]
    LeftSquare,

    #[token("]")]
    RightSquare,

    #[token("<")]
    LeftAngle,

    #[token(">")]
    RightAngle,

    #[token("?")]
    Question,

    #[token("!")]
    Bang,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("=")]
    Equal,

    #[token("<>")]
    NotEqual,

    #[token("<=")]
    LessEqual,

    #[token(">=")]
    GreaterEqual,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    Eoi,

    #[error]
    #[regex(r"[ \t\r\n\f\v]", logos::skip)]
    Error,
}

impl Token<'_> {
    fn into_identifier(self) -> String {
        match self {
            Self::Identifier(identifier) => identifier.to_owned(),
            _ => unreachable!(),
        }
    }

    fn into_boolean(self) -> bool {
        match self {
            Self::Boolean(boolean) => boolean,
            _ => unreachable!(),
        }
    }

    fn into_number(self) -> usize {
        match self {
            Self::Number(number) => number,
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
    Boolean,
    Number,
    LeftRound,
    RightRound,
    LeftBrace,
    RightBrace,
    LeftSquare,
    RightSquare,
    LeftAngle,
    RightAngle,
    Question,
    Bang,
    Colon,
    Comma,
    Equal,
    NotEqual,
    LessEqual,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Eoi,
    Error,
}

impl super::TokenId for TokenId {
    fn name(self) -> &'static str {
        match self {
            Self::Identifier => "an identifier",
            Self::Boolean => "a boolean",
            Self::Number => "a number",
            Self::LeftRound => "'('",
            Self::RightRound => "')'",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::LeftSquare => "'['",
            Self::RightSquare => "']'",
            Self::LeftAngle => "'<'",
            Self::RightAngle => "'>'",
            Self::Question => "'?'",
            Self::Bang => "'!'",
            Self::Colon => "':'",
            Self::Comma => "','",
            Self::Equal => "'='",
            Self::NotEqual => "'<>'",
            Self::LessEqual => "'<='",
            Self::GreaterEqual => "'>='",
            Self::Plus => "'+'",
            Self::Minus => "'-'",
            Self::Star => "'*'",
            Self::Slash => "'/'",
            Self::Eoi => "end of input",
            Self::Error => unreachable!(),
        }
    }
}

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("cannot mix named and unnamed parameters")]
    MixedParameters,
    #[error(transparent)]
    Expression(#[from] ExpressionError),
}

impl From<TransitionError> for ParseError {
    fn from(err: TransitionError) -> Self {
        FsmError::ParseTransition(err).into()
    }
}

enum Parameter<E> {
    Unnamed(String),
    Named(NamedParameter<String, E>),
}

impl<E> From<Parameter<E>> for Parameters<String, E> {
    fn from(parameter: Parameter<E>) -> Self {
        match parameter {
            Parameter::Unnamed(parameter) => Self::Unnamed(vec![parameter]),
            Parameter::Named(parameter) => Self::Named(vec![parameter]),
        }
    }
}

trait PushParameter<E> {
    fn push_parameter(&mut self, parameter: Parameter<E>) -> bool;
}

impl<E> PushParameter<E> for Parameters<String, E> {
    fn push_parameter(&mut self, parameter: Parameter<E>) -> bool {
        match (self, parameter) {
            (Parameters::Unnamed(parameters), Parameter::Unnamed(parameter)) => {
                parameters.push(parameter);
                true
            }
            (Parameters::Named(parameters), Parameter::Named(parameter)) => {
                parameters.push(parameter);
                true
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Brackets,
    Unary(UnaryOp),
    Binary(BinaryOp),
}

impl Operator for Op {
    fn associativity(&self) -> Associativity {
        match self {
            Op::Brackets => Associativity::Left,
            Op::Unary(op) => op.associativity(),
            Op::Binary(op) => op.associativity(),
        }
    }

    fn precedence(&self) -> usize {
        match self {
            Op::Brackets => 1,
            Op::Unary(op) => op.precedence(),
            Op::Binary(op) => op.precedence(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExpressionError {
    #[error("found an unclosed left bracket")]
    UnclosedBracket,
    #[error("operator is missing an operand")]
    MissingOperand,
    #[error("found an unexpected expression")]
    UnexpectedExpression,
}

impl From<ExpressionError> for ParseError {
    fn from(err: ExpressionError) -> Self {
        TransitionError::from(err).into()
    }
}

fn apply_operator(
    outputs: &mut Vec<Spanned<crate::Expression<String>>>,
    operator: Spanned<Op>,
) -> Result<(), Spanned<ExpressionError>> {
    let (span, operator) = operator.into_parts();
    let mut expression = || match operator {
        Op::Brackets => Err(ExpressionError::UnclosedBracket),
        Op::Unary(op) => {
            let output = outputs.pop().ok_or(ExpressionError::MissingOperand)?;
            let (output_span, output) = output.into_parts();
            Ok(Spanned::new(
                span.merge(&output_span),
                crate::Expression::Unary(op, output.into()),
            ))
        }
        Op::Binary(op) => {
            let right = outputs.pop().ok_or(ExpressionError::MissingOperand)?;
            let left = outputs.pop().ok_or(ExpressionError::MissingOperand)?;
            let ((right_span, right), (left_span, left)) = (right.into_parts(), left.into_parts());
            Ok(Spanned::new(
                left_span.merge(&right_span),
                crate::Expression::Binary(op, left.into(), right.into()),
            ))
        }
    };

    let expression = expression().map_err(|err| Spanned::new(span, err))?;
    outputs.push(expression);

    Ok(())
}

fn push_operator(
    outputs: &mut Vec<Spanned<crate::Expression<String>>>,
    operators: &mut Vec<Spanned<Op>>,
    operator: Spanned<Op>,
) -> Result<(), Spanned<ExpressionError>> {
    while let Some(other) = operators.last() {
        if other.inner == Op::Brackets {
            break;
        }

        if operator.precedence() <= other.precedence()
            && (operator.precedence() != other.precedence()
                || operator.associativity() != Associativity::Left)
        {
            break;
        }

        apply_operator(outputs, operators.pop().unwrap())?;
    }

    operators.push(operator);
    Ok(())
}

fn parse_unary_op(tokens: &mut Lexer) -> Option<Spanned<UnaryOp>> {
    if let Some(token) = tokens.next_if(TokenId::Bang) {
        return Some(token.map(|_| UnaryOp::Not));
    }

    None
}

fn parse_binary_op(tokens: &mut Lexer) -> Option<Spanned<BinaryOp>> {
    if let Some(token) = tokens.next_if(TokenId::LeftAngle) {
        return Some(token.map(|_| BinaryOp::Less));
    }

    if let Some(token) = tokens.next_if(TokenId::RightAngle) {
        return Some(token.map(|_| BinaryOp::Greater));
    }

    if let Some(token) = tokens.next_if(TokenId::Equal) {
        return Some(token.map(|_| BinaryOp::Equal));
    }

    if let Some(token) = tokens.next_if(TokenId::NotEqual) {
        return Some(token.map(|_| BinaryOp::NotEqual));
    }

    if let Some(token) = tokens.next_if(TokenId::LessEqual) {
        return Some(token.map(|_| BinaryOp::LessEqual));
    }

    if let Some(token) = tokens.next_if(TokenId::GreaterEqual) {
        return Some(token.map(|_| BinaryOp::GreaterEqual));
    }

    if let Some(token) = tokens.next_if(TokenId::Plus) {
        return Some(token.map(|_| BinaryOp::Add));
    }

    if let Some(token) = tokens.next_if(TokenId::Minus) {
        return Some(token.map(|_| BinaryOp::Subtract));
    }

    if let Some(token) = tokens.next_if(TokenId::Star) {
        return Some(token.map(|_| BinaryOp::Multiply));
    }

    if let Some(token) = tokens.next_if(TokenId::Slash) {
        return Some(token.map(|_| BinaryOp::Divide));
    }

    None
}

fn parse_expression(
    tokens: &mut Lexer,
    terminal: TokenId,
) -> Option<Option<Spanned<crate::Expression<String>>>> {
    let (mut outputs, mut operators) = (Vec::new(), Vec::new());
    'e: loop {
        if tokens.next_if(terminal).is_some() {
            break;
        }

        if let Some(token) = tokens.next_if(TokenId::Identifier) {
            outputs.push(token.map(|token| crate::Expression::Name(token.into_identifier())));
            continue;
        }

        if let Some(token) = tokens.next_if(TokenId::Boolean) {
            outputs.push(token.map(|token| crate::Expression::Boolean(token.into_boolean())));
            continue;
        }

        if let Some(token) = tokens.next_if(TokenId::Number) {
            outputs.push(token.map(|token| crate::Expression::Number(token.into_number())));
            continue;
        }

        if let Some(op) = parse_unary_op(tokens) {
            if let Err(err) = push_operator(&mut outputs, &mut operators, op.map(Op::Unary)) {
                tokens.push_err(err.span, err.inner.into());
                return None;
            }

            continue;
        }

        if let Some(op) = parse_binary_op(tokens) {
            if let Err(err) = push_operator(&mut outputs, &mut operators, op.map(Op::Binary)) {
                tokens.push_err(err.span, err.inner.into());
                return None;
            }

            continue;
        }

        if let Some(token) = tokens.next_if(TokenId::LeftRound) {
            operators.push(token.map(|_| Op::Brackets));
            continue;
        }

        let token = tokens.expect_next_if(TokenId::RightRound)?;
        while let Some(operator) = operators.pop() {
            if operator.inner == Op::Brackets {
                continue 'e;
            }

            if let Err(err) = apply_operator(&mut outputs, operator) {
                tokens.push_err(err.span, err.inner.into());
                return None;
            }
        }

        tokens.push_err(token.span, ExpressionError::UnclosedBracket.into());
        return None;
    }

    while let Some(operator) = operators.pop() {
        if let Err(err) = apply_operator(&mut outputs, operator) {
            tokens.push_err(err.span, err.inner.into());
            return None;
        }
    }

    let mut outputs = outputs.into_iter().peekable();
    let output = match outputs.next() {
        Some(output) => output,
        None => return Some(None),
    };

    if outputs.peek().is_some() {
        for output in outputs {
            tokens.push_err(output.span, ExpressionError::UnexpectedExpression.into());
        }

        return None;
    }

    Some(Some(output))
}

pub(super) trait Expression: Eq + Hash + Sized {
    fn parse_refinement(_: &mut Lexer) -> Option<Option<Spanned<Self>>> {
        Some(None)
    }

    fn parse_assignments(_: &mut Lexer) -> Option<Vec<(String, Self)>> {
        Some(Vec::new())
    }
}

impl Expression for Infallible {}

impl Expression for crate::Expression<String> {
    fn parse_refinement(tokens: &mut Lexer) -> Option<Option<Spanned<Self>>> {
        if tokens.next_if(TokenId::LeftBrace).is_some() {
            return parse_expression(tokens, TokenId::RightBrace);
        }

        Some(None)
    }
}

fn parse_parameter<E: Expression>(tokens: &mut Lexer) -> Option<Spanned<Parameter<E>>> {
    let name = tokens.expect_next_if(TokenId::Identifier)?;
    let name = name.map(Token::into_identifier);

    if tokens.next_if(TokenId::Colon).is_some() {
        let sort = tokens.expect_next_if(TokenId::Identifier)?;
        let (mut span, sort) = sort.map(Token::into_identifier).into_parts();

        let refinement = E::parse_refinement(tokens)?.map(|refinement| {
            span = refinement.span;
            refinement.inner
        });

        let parameter = Parameter::Named(NamedParameter::new(name.inner, sort, refinement));
        return Some(Spanned::new(name.span.merge(&span), parameter));
    }

    Some(name.map(Parameter::Unnamed))
}

fn parse_parameters<E: Expression>(tokens: &mut Lexer) -> Option<Option<Parameters<String, E>>> {
    if tokens.next_if(TokenId::RightRound).is_some() {
        return Some(None);
    }

    let parameter = parse_parameter(tokens)?;
    let mut parameters = Parameters::from(parameter.inner);

    loop {
        if tokens.next_if(TokenId::RightRound).is_some() {
            break;
        }

        tokens.expect_next_if(TokenId::Comma)?;
        if tokens.next_if(TokenId::RightRound).is_some() {
            break;
        }

        let parameter = parse_parameter(tokens)?;
        if !parameters.push_parameter(parameter.inner) {
            tokens.push_err(parameter.span, TransitionError::MixedParameters.into())
        }
    }

    Some(Some(parameters))
}

pub(super) fn parse<E: Expression>(tokens: &mut Lexer) -> Option<Transition<String, String, E>> {
    let role = tokens.expect_next_if(TokenId::Identifier)?;
    let role = role.map(Token::into_identifier);

    let action = match tokens.next_if(TokenId::Question) {
        Some(_) => Action::Input,
        None => {
            tokens.expect_next_if(TokenId::Bang)?;
            Action::Output
        }
    };

    let label = tokens.expect_next_if(TokenId::Identifier)?;
    let label = label.map(Token::into_identifier);

    let (mut parameters, assignments) = (None, Default::default());
    if tokens.next_if(TokenId::LeftRound).is_some() {
        parameters = parse_parameters(tokens)?;
        if tokens.next_if(TokenId::LeftSquare).is_some() {
            unimplemented!("refinements are not yet implemented");
        }
    }

    tokens.expect_next_if(TokenId::Eoi);

    let message = Message::new(label.inner, parameters.unwrap_or_default(), assignments);
    Some(Transition::new(role.inner, action, message))
}
