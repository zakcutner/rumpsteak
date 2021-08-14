use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    try_join,
};
use rumpsteak::{
    channel::{Bidirectional, Nil},
    session, try_session, End, Message, Receive, Role, Roles, Send,
};
use std::{error::Error, marker, result, sync::Arc};

type Result<T> = result::Result<T, Box<dyn Error + marker::Send + marker::Sync>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(S, K, T);

#[derive(Role)]
#[message(Label)]
struct S(#[route(K)] Channel, #[route(T)] Nil);

#[derive(Role)]
#[message(Label)]
struct K(#[route(S)] Channel, #[route(T)] Channel);

#[derive(Role)]
#[message(Label)]
struct T(#[route(S)] Nil, #[route(K)] Channel);

#[derive(Message)]
enum Label {
    Ready(Ready),
    Value(Value),
}

struct Ready;
struct Value(Vec<i32>);

#[session]
type Source = Receive<K, Ready, Send<K, Value, Receive<K, Ready, Send<K, Value, End>>>>;

#[session]
#[rustfmt::skip]
type Kernel = Send<S, Ready, Receive<S, Value, Receive<T, Ready, Send<T, Value, Send<S, Ready, Receive<S, Value, Receive<T, Ready, Send<T, Value, End>>>>>>>>;

#[session]
#[rustfmt::skip]
type KernelOptimized = Send<S, Ready, Send<S, Ready, Receive<S, Value, Receive<T, Ready, Send<T, Value, Receive<S, Value, Receive<T, Ready, Send<T, Value, End>>>>>>>>;

#[session]
type Sink = Send<K, Ready, Receive<K, Value, Send<K, Ready, Receive<K, Value, End>>>>;

async fn source(role: &mut S, values: &[i32]) -> Result<()> {
    try_session(role, |s: Source<'_, _>| async {
        let half = values.len() / 2;

        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values[..half].to_vec())).await?;

        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values[half..].to_vec())).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel(role: &mut K) -> Result<()> {
    try_session(role, |s: Kernel<'_, _>| async {
        let s = s.send(Ready).await?;
        let (Value(values), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values)).await?;

        let s = s.send(Ready).await?;
        let (Value(values), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values)).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel_optimized(role: &mut K) -> Result<()> {
    try_session(role, |s: KernelOptimized<'_, _>| async {
        let s = s.send(Ready).await?;
        let s = s.send(Ready).await?;

        let (Value(values), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values)).await?;

        let (Value(values), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Value(values)).await?;

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<Vec<i32>> {
    try_session(role, |s: Sink<'_, _>| async {
        let mut output = Vec::new();

        let s = s.send(Ready).await?;
        let (Value(values), s) = s.receive().await?;
        output.extend(values);

        let s = s.send(Ready).await?;
        let (Value(values), s) = s.receive().await?;
        output.extend(values);

        Ok((output, s))
    })
    .await
}

pub async fn run(input: Arc<[i32]>) {
    let Roles(mut s, mut k, mut t) = Roles::default();
    let (_, _, output) = try_join!(
        {
            let input = input.clone();
            tokio::spawn(async move { source(&mut s, &input).await.unwrap() })
        },
        tokio::spawn(async move { kernel(&mut k).await.unwrap() }),
        tokio::spawn(async move { sink(&mut t).await.unwrap() }),
    )
    .unwrap();
    assert_eq!(input.as_ref(), output.as_slice());
}

pub async fn run_optimized(input: Arc<[i32]>) {
    let Roles(mut s, mut k, mut t) = Roles::default();
    let (_, _, output) = try_join!(
        {
            let input = input.clone();
            tokio::spawn(async move { source(&mut s, &input).await.unwrap() })
        },
        tokio::spawn(async move { kernel_optimized(&mut k).await.unwrap() }),
        tokio::spawn(async move { sink(&mut t).await.unwrap() }),
    )
    .unwrap();
    assert_eq!(input.as_ref(), output.as_slice());
}
