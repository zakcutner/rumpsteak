# :meat_on_bone: Rumpsteak

[![Actions](https://github.com/zakcutner/rumpsteak/workflows/Check/badge.svg)](https://github.com/zakcutner/rumpsteak/actions)
[![Crate](https://img.shields.io/crates/v/rumpsteak)](https://crates.io/crates/rumpsteak)
[![Docs](https://docs.rs/rumpsteak/badge.svg)](https://docs.rs/rumpsteak)
[![License](https://img.shields.io/crates/l/rumpsteak)](LICENSE)

> :warning: Rumpsteak is currently a work in progress and the API is likely to
> dramatically change. Feel free to try out examples but please do not yet use
> Rumpsteak for any production applications!

Rumpsteak is a Rust framework for _safely_ and _efficiently_ implementing
[message-passing](https://doc.rust-lang.org/book/ch16-02-message-passing.html)
[asynchronous](https://rust-lang.github.io/async-book/) programs. It uses
multiparty session types to statically guarantee the absence of communication
errors such as deadlocks.

Multiparty session types verify the safety of message-passing protocols, as
described in
[A Gentle Introduction to Multiparty Asynchronous Session Types](http://mrg.doc.ic.ac.uk/publications/a-gentle-introduction-to-multiparty-asynchronous-session-types/paper.pdf).
Code generation of protocols into Rumpsteak's API using the
[νScr toolkit](https://github.com/nuscr/nuscr) is currently
[in progress](generate).

## Features

- [x] Provides deadlock-free communication.
- [x] Integrates with `async`/`await` code.
- [x] Supports any number of participants.
- [x] Includes benchmarks to track performance.

## Usage

Add the following to your `Cargo.toml` file.

```toml
[dependencies]
rumpsteak = "0.1"
```

## Example

```rust
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{channel::Bidirectional, try_session, End, Message, Receive, Role, Roles, Send};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Label)]
struct C(#[route(S)] Channel);

#[derive(Role)]
#[message(Label)]
struct S(#[route(C)] Channel);

#[derive(Message)]
enum Label {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

type Client = Send<S, Add, Send<S, Add, Receive<S, Sum, End>>>;

type Server = Receive<C, Add, Receive<C, Add, Send<C, Sum, End>>>;

async fn client(role: &mut C, x: i32, y: i32) -> Result<i32> {
    try_session(|s: Client| async {
        let s = s.send(role, Add(x)).await?;
        let s = s.send(role, Add(y)).await?;
        let (Sum(z), s) = s.receive(role).await?;
        Ok((z, s))
    })
    .await
}

async fn server(role: &mut S) -> Result<()> {
    try_session(|s: Server| async {
        let (Add(x), s) = s.receive(role).await?;
        let (Add(y), s) = s.receive(role).await?;
        let s = s.send(role, Sum(x + y)).await?;
        Ok(((), s))
    })
    .await
}

fn main() {
    let Roles(mut c, mut s) = Roles::default();
    executor::block_on(async {
        let (output, _) = try_join!(client(&mut c, 1, 2), server(&mut s)).unwrap();
        assert_eq!(output, 3);
    });
}
```

## Licensing

Licensed under the MIT license. See the [LICENSE](LICENSE) file for details.
