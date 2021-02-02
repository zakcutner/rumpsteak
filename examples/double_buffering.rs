#![allow(clippy::type_complexity)]

use futures::{
    executor::{self, ThreadPool},
    try_join, FutureExt,
};
use session::{
    choice::Choice,
    role::{Nil, Role, Roles, Route, ToFrom},
    try_session, Branch, End, Label, Receive, Result, Select, Send,
};
use std::{future::Future, marker};

#[derive(Roles)]
struct Roles(A, B, S, T);

#[derive(Role)]
#[message(Message)]
struct A(
    #[route(B)] Nil<B>,
    #[route(S)] ToFrom<S>,
    #[route(T)] ToFrom<T>,
);

#[derive(Role)]
#[message(Message)]
struct B(
    #[route(A)] Nil<A>,
    #[route(S)] ToFrom<S>,
    #[route(T)] ToFrom<T>,
);

#[derive(Role)]
#[message(Message)]
struct S(
    #[route(A)] ToFrom<A>,
    #[route(B)] ToFrom<B>,
    #[route(T)] Nil<T>,
);

#[derive(Role)]
#[message(Message)]
struct T(
    #[route(A)] ToFrom<A>,
    #[route(B)] ToFrom<B>,
    #[route(S)] Nil<S>,
);

#[derive(Label)]
enum Message {
    Ready(Ready),
    Stop(Stop),
    Copy(Copy),
}

struct Ready;
struct Stop;
struct Copy(i32);

#[rustfmt::skip]
type Buffer<'r, R> = Send<'r, R, S, Ready, Branch<'r, R, S, BufferChoice<'r, R>>>;

#[derive(Choice)]
#[role('r, R)]
enum BufferChoice<'r, R>
where
    R: Route<S, Route = ToFrom<S>> + Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    S: Route<R, Route = ToFrom<R>>,
    T: Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Receive<'r, R, T, Ready, Send<'r, R, T, Stop, End<'r>>>),
    #[rustfmt::skip]
    Copy(Copy, Receive<'r, R, T, Ready, Send<'r, R, T, Copy, Buffer<'r, R>>>),
}

#[rustfmt::skip]
type Source<'s> = Receive<'s, S, A, Ready, Select<'s, S, A, SourceChoice<'s, B, A>>>;

#[derive(Choice)]
#[role('s, S)]
enum SourceChoice<'s, Q, R>
where
    Q: Route<S, Route = ToFrom<S>> + Role<Message = Message>,
    R: Route<S, Route = ToFrom<S>> + Role<Message = Message>,
    S: Route<Q, Route = ToFrom<Q>> + Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Receive<'s, S, Q, Ready, Send<'s, S, Q, Stop, End<'s>>>),
    #[rustfmt::skip]
    Copy(Copy, Receive<'s, S, Q, Ready, Select<'s, S, Q, SourceChoice<'s, R, Q>>>),
}

#[rustfmt::skip]
type Sink<'t> = Send<'t, T, A, Ready, Branch<'t, T, A, SinkChoice<'t, B, A>>>;

#[derive(Choice)]
#[role('t, T)]
enum SinkChoice<'t, Q, R>
where
    Q: Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    R: Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    T: Route<Q, Route = ToFrom<Q>> + Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Send<'t, T, Q, Ready, Receive<'t, T, Q, Stop, End<'t>>>),
    #[rustfmt::skip]
    Copy(Copy, Send<'t, T, Q, Ready, Branch<'t, T, Q, SinkChoice<'t, R, Q>>>),
}

async fn buffer<R>(role: &mut R) -> Result<()>
where
    R: Route<S, Route = ToFrom<S>> + Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    S: Route<R, Route = ToFrom<R>>,
    T: Route<R, Route = ToFrom<R>>,
{
    try_session(role, |mut s: Buffer<'_, R>| async {
        let s = loop {
            s = match s.send(Ready)?.branch().await? {
                BufferChoice::Stop(Stop, s) => {
                    let (Ready, s) = s.receive().await?;
                    break s.send(Stop)?;
                }
                BufferChoice::Copy(Copy(v), s) => {
                    let (Ready, s) = s.receive().await?;
                    s.send(Copy(v))?
                }
            };
        };

        Ok(((), s))
    })
    .await
}

async fn source(role: &mut S, values: &[i32]) -> Result<()> {
    try_session(role, |mut s: Source<'_>| async {
        let mut values = values.iter();
        let s = loop {
            s = {
                let (Ready, s) = s.receive().await?;
                let s = match values.next() {
                    Some(&v) => s.select(Copy(v))?,
                    None => {
                        let s = s.select(Stop)?;
                        let (Ready, s) = s.receive().await?;
                        break s.send(Stop)?;
                    }
                };

                let (Ready, s) = s.receive().await?;
                match values.next() {
                    Some(&v) => s.select(Copy(v))?,
                    None => {
                        let s = s.select(Stop)?;
                        let (Ready, s) = s.receive().await?;
                        break s.send(Stop)?;
                    }
                }
            };
        };

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<Vec<i32>> {
    try_session(role, |mut s: Sink<'_>| async {
        let mut values = vec![];
        Ok(loop {
            s = {
                let s = s.send(Ready)?;
                let s = match s.branch().await? {
                    SinkChoice::Stop(Stop, s) => {
                        let s = s.send(Ready)?;
                        let (Stop, s) = s.receive().await?;
                        break (values, s);
                    }
                    SinkChoice::Copy(Copy(v), s) => {
                        values.push(v);
                        s
                    }
                };

                let s = s.send(Ready)?;
                match s.branch().await? {
                    SinkChoice::Stop(Stop, s) => {
                        let s = s.send(Ready)?;
                        let (Stop, s) = s.receive().await?;
                        break (values, s);
                    }
                    SinkChoice::Copy(Copy(v), s) => {
                        values.push(v);
                        s
                    }
                }
            };
        })
    })
    .await
}

async fn spawn<F: Future + marker::Send + 'static>(pool: &ThreadPool, future: F) -> F::Output
where
    F::Output: marker::Send,
{
    let (future, handle) = future.remote_handle();
    pool.spawn_ok(future);
    handle.await
}

fn main() {
    let Roles(mut a, mut b, mut s, mut t) = Roles::default();
    let pool = ThreadPool::new().unwrap();

    let input = &[1, 2, 3, 4, 5];
    println!("input = {:?}", input);

    let (_, _, _, output) = executor::block_on(async {
        try_join!(
            spawn(&pool, async move { buffer(&mut a).await }),
            spawn(&pool, async move { buffer(&mut b).await }),
            spawn(&pool, async move { source(&mut s, input).await }),
            spawn(&pool, async move { sink(&mut t).await }),
        )
        .unwrap()
    });

    println!("output = {:?}", output);
    assert_eq!(input, output.as_slice());
}
