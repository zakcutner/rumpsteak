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
use futures::{executor, try_join};
use rumpsteak::{
    role::{Role, Roles, ToFrom},
    try_session, End, Label, Receive, Result, Send,
};

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Message)]
struct C(#[route(S)] ToFrom<S>);

#[derive(Role)]
#[message(Message)]
struct S(#[route(C)] ToFrom<C>);

#[derive(Label)]
enum Message {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

type Client<'c> = Send<'c, C, S, Add, Send<'c, C, S, Add, Receive<'c, C, S, Sum, End<'c>>>>;

type Server<'s> = Receive<'s, S, C, Add, Receive<'s, S, C, Add, Send<'s, S, C, Sum, End<'s>>>>;

async fn client(role: &mut C, x: i32, y: i32) -> Result<i32> {
    try_session(role, |s: Client<'_>| async {
        let s = s.send(Add(x))?;
        let s = s.send(Add(y))?;
        let (Sum(z), s) = s.receive().await?;
        Ok((z, s))
    })
    .await
}

async fn server(role: &mut S) -> Result<()> {
    try_session(role, |s: Server<'_>| async {
        let (Add(x), s) = s.receive().await?;
        let (Add(y), s) = s.receive().await?;
        let s = s.send(Sum(x + y))?;
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
