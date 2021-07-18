use futures::{channel::oneshot, try_join, Future, Sink, Stream};
use num_complex::Complex32;
use rumpsteak::{
    channel::{Bidirectional, Nil, Pair},
    session, try_session, End, Message, Receive, Role, Roles, Send,
};
use std::{
    error::Error,
    marker,
    pin::Pin,
    result,
    sync::Arc,
    task::{Context, Poll},
};

type Result<T, E = Box<dyn Error + marker::Send + marker::Sync>> = result::Result<T, E>;

type Channel = Bidirectional<Sender<Value>, Receiver<Value>>;

struct Sender<T>(Option<oneshot::Sender<T>>);

impl<T> Pair<Receiver<T>> for Sender<T> {
    fn pair() -> (Self, Receiver<T>) {
        let (sender, receiver) = oneshot::channel();
        (Sender(Some(sender)), Receiver(receiver))
    }
}

impl<T> Sender<T> {
    fn inner(self: Pin<&mut Self>) -> Pin<&mut Option<oneshot::Sender<T>>> {
        Pin::new(&mut self.get_mut().0)
    }
}

impl<T> Sink<T> for Sender<T> {
    type Error = &'static str;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        match self.inner().take() {
            Some(channel) => {
                let message = "could not send since the reveiving end of the channel was dropped";
                channel.send(item).map_err(|_| message)
            }
            None => Err("could not send since the channel was already used"),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

struct Receiver<T>(oneshot::Receiver<T>);

impl<T> Pair<Sender<T>> for Receiver<T> {
    fn pair() -> (Self, Sender<T>) {
        let (sender, receiver) = Pair::pair();
        (receiver, sender)
    }
}

impl<T> Receiver<T> {
    fn inner(self: Pin<&mut Self>) -> Pin<&mut oneshot::Receiver<T>> {
        Pin::new(&mut self.get_mut().0)
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner().poll(cx) {
            Poll::Ready(next) => Poll::Ready(next.ok()),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Roles)]
struct Roles(R0, R1, R2, R3, R4, R5, R6, R7);

#[derive(Role)]
#[message(Value)]
struct R0(
    #[route(R1)] Channel,
    #[route(R2)] Channel,
    #[route(R3)] Nil,
    #[route(R4)] Channel,
    #[route(R5)] Nil,
    #[route(R6)] Nil,
    #[route(R7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct R1(
    #[route(R0)] Channel,
    #[route(R2)] Nil,
    #[route(R3)] Channel,
    #[route(R4)] Nil,
    #[route(R5)] Channel,
    #[route(R6)] Nil,
    #[route(R7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct R2(
    #[route(R0)] Channel,
    #[route(R1)] Nil,
    #[route(R3)] Channel,
    #[route(R4)] Nil,
    #[route(R5)] Nil,
    #[route(R6)] Channel,
    #[route(R7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct R3(
    #[route(R0)] Nil,
    #[route(R1)] Channel,
    #[route(R2)] Channel,
    #[route(R4)] Nil,
    #[route(R5)] Nil,
    #[route(R6)] Nil,
    #[route(R7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct R4(
    #[route(R0)] Channel,
    #[route(R1)] Nil,
    #[route(R2)] Nil,
    #[route(R3)] Nil,
    #[route(R5)] Channel,
    #[route(R6)] Channel,
    #[route(R7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct R5(
    #[route(R0)] Nil,
    #[route(R1)] Channel,
    #[route(R2)] Nil,
    #[route(R3)] Nil,
    #[route(R4)] Channel,
    #[route(R6)] Nil,
    #[route(R7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct R6(
    #[route(R0)] Nil,
    #[route(R1)] Nil,
    #[route(R2)] Channel,
    #[route(R3)] Nil,
    #[route(R4)] Channel,
    #[route(R5)] Nil,
    #[route(R7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct R7(
    #[route(R0)] Nil,
    #[route(R1)] Nil,
    #[route(R2)] Nil,
    #[route(R3)] Channel,
    #[route(R4)] Nil,
    #[route(R5)] Channel,
    #[route(R6)] Channel,
);

#[derive(Message)]
struct Value(Complex32);

#[session]
type Butterfly<R, S> = Send<R, Value, Receive<R, Value, S>>;

#[session]
type ButterflyR<R, S> = Receive<R, Value, Send<R, Value, S>>;

#[session]
type Butterfly0 = Butterfly<R4, Butterfly<R2, Butterfly<R1, End>>>;

async fn butterfly_0(role: &mut R0, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly0<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

async fn butterfly_0_optimized(role: &mut R0, x: Complex32) -> Result<Complex32> {
    butterfly_0(role, x).await
}

#[session]
type Butterfly1 = Butterfly<R5, Butterfly<R3, ButterflyR<R0, End>>>;

#[session]
type Butterfly1Optimized = Butterfly<R5, Butterfly<R3, Butterfly<R0, End>>>;

async fn butterfly_1(role: &mut R1, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly1<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

async fn butterfly_1_optimized(role: &mut R1, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly1Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly2 = Butterfly<R6, ButterflyR<R0, Butterfly<R3, End>>>;

#[session]
type Butterfly2Optimized = Butterfly<R6, Butterfly<R0, Butterfly<R3, End>>>;

async fn butterfly_2(role: &mut R2, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly2<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

async fn butterfly_2_optimized(role: &mut R2, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly2Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly3 = Butterfly<R7, ButterflyR<R1, ButterflyR<R2, End>>>;

#[session]
type Butterfly3Optimized = Butterfly<R7, Butterfly<R1, Butterfly<R2, End>>>;

async fn butterfly_3(role: &mut R3, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly3<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = crate::rotate_90(y - x);

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

async fn butterfly_3_optimized(role: &mut R3, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly3Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_90(y - x);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly4 = ButterflyR<R0, Butterfly<R6, Butterfly<R5, End>>>;

#[session]
type Butterfly4Optimized = Butterfly<R0, Butterfly<R6, Butterfly<R5, End>>>;

async fn butterfly_4(role: &mut R4, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly4<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

async fn butterfly_4_optimized(role: &mut R4, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly4Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly5 = ButterflyR<R1, Butterfly<R7, ButterflyR<R4, End>>>;

#[session]
type Butterfly5Optimized = Butterfly<R1, Butterfly<R7, Butterfly<R4, End>>>;

async fn butterfly_5(role: &mut R5, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly5<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_45(x + y);

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

async fn butterfly_5_optimized(role: &mut R5, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly5Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_45(x + y);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly6 = ButterflyR<R2, ButterflyR<R4, Butterfly<R7, End>>>;

#[session]
type Butterfly6Optimized = Butterfly<R2, Butterfly<R4, Butterfly<R7, End>>>;

async fn butterfly_6(role: &mut R6, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly6<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = crate::rotate_90(y - x);

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

async fn butterfly_6_optimized(role: &mut R6, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly6Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_90(y - x);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = x + y;

        Ok((x, s))
    })
    .await
}

#[session]
type Butterfly7 = ButterflyR<R3, ButterflyR<R5, ButterflyR<R6, End>>>;

#[session]
type Butterfly7Optimized = Butterfly<R3, Butterfly<R5, Butterfly<R6, End>>>;

async fn butterfly_7(role: &mut R7, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly7<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = crate::rotate_90(y - x);

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = crate::rotate_135(y - x);

        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x)).await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

async fn butterfly_7_optimized(role: &mut R7, x: Complex32) -> Result<Complex32> {
    try_session(role, |s: Butterfly7Optimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_90(y - x);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = crate::rotate_135(y - x);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = y - x;

        Ok((x, s))
    })
    .await
}

pub async fn run(input: Arc<[Complex32]>) -> Vec<Complex32> {
    let mut futures = Vec::with_capacity(input.len() / 8);
    let mut output = Vec::with_capacity(input.len());

    for i in (0..input.len()).step_by(8) {
        let Roles(mut r0, mut r1, mut r2, mut r3, mut r4, mut r5, mut r6, mut r7) =
            Roles::default();
        let input = input.clone();

        futures.push(tokio::spawn(async move {
            let (o0, o4, o2, o6, o1, o5, o3, o7) = try_join!(
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_0(&mut r0, input[i]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_1(&mut r1, input[i + 1]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_2(&mut r2, input[i + 2]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_3(&mut r3, input[i + 3]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_4(&mut r4, input[i + 4]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_5(&mut r5, input[i + 5]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_6(&mut r6, input[i + 6]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_7(&mut r7, input[i + 7]).await.unwrap() }
                }),
            )
            .unwrap();

            [o0, o1, o2, o3, o4, o5, o6, o7]
        }));
    }

    for future in futures {
        output.extend(future.await.unwrap());
    }

    output
}

pub async fn run_optimized(input: Arc<[Complex32]>) -> Vec<Complex32> {
    let mut futures = Vec::with_capacity(input.len() / 8);
    let mut output = Vec::with_capacity(input.len());

    for i in (0..input.len()).step_by(8) {
        let Roles(mut r0, mut r1, mut r2, mut r3, mut r4, mut r5, mut r6, mut r7) =
            Roles::default();
        let input = input.clone();

        futures.push(tokio::spawn(async move {
            let (o0, o4, o2, o6, o1, o5, o3, o7) = try_join!(
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_0_optimized(&mut r0, input[i]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_1_optimized(&mut r1, input[i + 1]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_2_optimized(&mut r2, input[i + 2]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_3_optimized(&mut r3, input[i + 3]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_4_optimized(&mut r4, input[i + 4]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_5_optimized(&mut r5, input[i + 5]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_6_optimized(&mut r6, input[i + 6]).await.unwrap() }
                }),
                tokio::spawn({
                    let input = input.clone();
                    async move { butterfly_7_optimized(&mut r7, input[i + 7]).await.unwrap() }
                }),
            )
            .unwrap();

            [o0, o1, o2, o3, o4, o5, o6, o7]
        }));
    }

    for future in futures {
        output.extend(future.await.unwrap());
    }

    output
}
