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
errors such as deadlocks and asynchronous subtyping to allow optimizing
communications.

Multiparty session types (MPST) verify the safety of message-passing protocols,
as described in [A Very Gentle Introduction to Multiparty Session
Types](http://mrg.doc.ic.ac.uk/publications/a-very-gentle-introduction-to-multiparty-session-types/main.pdf).
Asynchronous subtyping, introduced for MPST in [Precise Subtyping for
Asynchronous Multiparty
Sessions](http://mrg.doc.ic.ac.uk/publications/precise-subtyping-for-asynchronous-multiparty-sessions/main.pdf),
verifies the reordering of messages to create more optimized implementations
than are usually possible with MPST.

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
use rumpsteak::{
    channel::Bidirectional, session, try_session, End, Message, Receive, Role, Roles, Send,
};
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

#[session]
type Client = Send<S, Add, Send<S, Add, Receive<S, Sum, End>>>;

#[session]
type Server = Receive<C, Add, Receive<C, Add, Send<C, Sum, End>>>;

async fn client(role: &mut C, x: i32, y: i32) -> Result<i32> {
    try_session(role, |s: Client<'_, _>| async {
        let s = s.send(Add(x)).await?;
        let s = s.send(Add(y)).await?;
        let (Sum(z), s) = s.receive().await?;
        Ok((z, s))
    })
    .await
}

async fn server(role: &mut S) -> Result<()> {
    try_session(role, |s: Server<'_, _>| async {
        let (Add(x), s) = s.receive().await?;
        let (Add(y), s) = s.receive().await?;
        let s = s.send(Sum(x + y)).await?;
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

## Structure

#### `benches/`

Benchmark suite to track Rumpsteak's performance over time.

#### `comparison/`

Comparison with some other Rust implementations of session types.

#### `examples/`

Many examples of using Rumpsteak from popular protocols.

#### `generate/`

Automatic code generation from finite state machines to Rumpsteak's API.

#### `macros/`

Crate for procedural macros used within Rumpsteak's API.

#### `oneshot/`

Outdated experimental implementation of using one-shot channels for communication.

## Licensing

Licensed under the MIT license. See the [LICENSE](LICENSE) file for details.
