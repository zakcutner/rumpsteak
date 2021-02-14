#![allow(clippy::type_complexity)]

use futures::{
    executor::{self, ThreadPool},
    try_join, FutureExt,
};
use session::{
    role::{Nil, Role, Roles, ToFrom},
    try_session, End, Label, Receive, Result, Send,
};
use std::{future::Future, marker};

#[derive(Roles)]
struct Roles(S, K, T);

#[derive(Role)]
#[message(Message)]
struct S(#[route(K)] ToFrom<K>, #[route(T)] Nil<T>);

#[derive(Role)]
#[message(Message)]
struct K(#[route(S)] ToFrom<S>, #[route(T)] ToFrom<T>);

#[derive(Role)]
#[message(Message)]
struct T(#[route(S)] Nil<S>, #[route(K)] ToFrom<K>);

#[derive(Label)]
enum Message {
    Ready(Ready),
    Copy(Copy),
}

struct Ready;
struct Copy(i32);

#[rustfmt::skip]
type Source<'s> = Receive<'s, S, K, Ready, Send<'s, S, K, Copy, Receive<'s, S, K, Ready, Send<'s, S, K, Copy, End<'s>>>>>;

#[rustfmt::skip]
type Kernel<'k> = Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, End<'k>>>>>>>>>;

#[rustfmt::skip]
type Sink<'t> = Send<'t, T, K, Ready, Receive<'t, T, K, Copy, Send<'t, T, K, Ready, Receive<'t, T, K, Copy, End<'t>>>>>;

async fn source(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(role, |s: Source<'_>| async {
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.0))?;

        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.1))?;

        Ok(((), s))
    })
    .await
}

async fn kernel(role: &mut K) -> Result<()> {
    try_session(role, |s: Kernel<'_>| async {
        let s = s.send(Ready)?;
        let (Copy(x), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(x))?;

        let s = s.send(Ready)?;
        let (Copy(y), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(y))?;

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<(i32, i32)> {
    try_session(role, |s: Sink<'_>| async {
        let s = s.send(Ready)?;
        let (Copy(x), s) = s.receive().await?;

        let s = s.send(Ready)?;
        let (Copy(y), s) = s.receive().await?;

        Ok(((x, y), s))
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
    let Roles(mut s, mut k, mut t) = Roles::default();
    let pool = ThreadPool::new().unwrap();

    let input = (1, 2);
    println!("input = {:?}", input);

    let (_, _, output) = executor::block_on(async {
        try_join!(
            spawn(&pool, async move { source(&mut s, input).await }),
            spawn(&pool, async move { kernel(&mut k).await }),
            spawn(&pool, async move { sink(&mut t).await }),
        )
        .unwrap()
    });

    println!("output = {:?}", output);
    assert_eq!(input, output);
}
