use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    try_join,
};
use rumpsteak::{
    channel::Bidirectional, session, try_session, Branch, End, Message, Receive, Role, Roles,
    Select, Send,
};
use std::{error::Error, result, sync::Arc};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(S, T);

#[derive(Role)]
#[message(Label)]
struct S(#[route(T)] Channel);

#[derive(Role)]
#[message(Label)]
struct T(#[route(S)] Channel);

#[derive(Message)]
enum Label {
    Ready(Ready),
    Value(Value),
    Stop(Stop),
}

struct Ready;
struct Value(i32);
struct Stop;

#[session]
type Source = Receive<T, Ready, Select<T, SourceChoice>>;

#[session]
enum SourceChoice {
    Value(Value, Source),
    Stop(Stop, End),
}

#[session]
type Sink = Send<S, Ready, Branch<S, SinkChoice>>;

#[session]
enum SinkChoice {
    Value(Value, Sink),
    Stop(Stop, End),
}

async fn source_inner(
    s: Source<'_, S>,
    values: impl Iterator<Item = i32>,
) -> Result<((), End<'_, S>)> {
    let (Ready, mut s) = s.receive().await?;
    for value in values {
        s = {
            let s = s.select(Value(value)).await?;
            let (Ready, s) = s.receive().await?;
            s
        };
    }

    Ok(((), s.select(Stop).await?))
}

async fn source(role: &mut S, values: &[i32]) -> Result<()> {
    try_session(role, |s: Source<'_, _>| async {
        source_inner(s, values.iter().copied()).await
    })
    .await
}

async fn sink(role: &mut T) -> Result<Vec<i32>> {
    try_session(role, |mut s: Sink<'_, _>| async {
        let mut output = Vec::new();
        loop {
            s = {
                let s = s.send(Ready).await?;
                match s.branch().await? {
                    SinkChoice::Value(Value(value), s) => {
                        output.push(value);
                        s
                    }
                    SinkChoice::Stop(Stop, s) => return Ok((output, s)),
                }
            }
        }
    })
    .await
}

pub async fn run(input: Arc<[i32]>) {
    let Roles(mut s, mut t) = Roles::default();
    let (_, output) = try_join!(source(&mut s, &input), sink(&mut t)).unwrap();
    assert_eq!(input.as_ref(), output.as_slice());
}

include!(concat!(env!("OUT_DIR"), "/rumpsteak.rs"));
