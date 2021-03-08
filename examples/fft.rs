use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use num_complex::{Complex, Complex32};
use rumpsteak::{
    channel::{Bidirectional, Nil},
    session, try_session, End, Message, Receive, Role, Roles, Route, Send,
};
use std::{
    error::Error,
    f32::consts::PI,
    fmt::{self, Display, Formatter},
    result,
};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Value>, UnboundedReceiver<Value>>;

#[derive(Roles)]
struct Roles(P0, P1, P2, P3, P4, P5, P6, P7);

#[derive(Role)]
#[message(Value)]
struct P0(
    #[route(P1)] Channel,
    #[route(P2)] Channel,
    #[route(P3)] Nil,
    #[route(P4)] Channel,
    #[route(P5)] Nil,
    #[route(P6)] Nil,
    #[route(P7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct P1(
    #[route(P0)] Channel,
    #[route(P2)] Nil,
    #[route(P3)] Channel,
    #[route(P4)] Nil,
    #[route(P5)] Channel,
    #[route(P6)] Nil,
    #[route(P7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct P2(
    #[route(P0)] Channel,
    #[route(P1)] Nil,
    #[route(P3)] Channel,
    #[route(P4)] Nil,
    #[route(P5)] Nil,
    #[route(P6)] Channel,
    #[route(P7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct P3(
    #[route(P0)] Nil,
    #[route(P1)] Channel,
    #[route(P2)] Channel,
    #[route(P4)] Nil,
    #[route(P5)] Nil,
    #[route(P6)] Nil,
    #[route(P7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct P4(
    #[route(P0)] Channel,
    #[route(P1)] Nil,
    #[route(P2)] Nil,
    #[route(P3)] Nil,
    #[route(P5)] Channel,
    #[route(P6)] Channel,
    #[route(P7)] Nil,
);

#[derive(Role)]
#[message(Value)]
struct P5(
    #[route(P0)] Nil,
    #[route(P1)] Channel,
    #[route(P2)] Nil,
    #[route(P3)] Nil,
    #[route(P4)] Channel,
    #[route(P6)] Nil,
    #[route(P7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct P6(
    #[route(P0)] Nil,
    #[route(P1)] Nil,
    #[route(P2)] Channel,
    #[route(P3)] Nil,
    #[route(P4)] Channel,
    #[route(P5)] Nil,
    #[route(P7)] Channel,
);

#[derive(Role)]
#[message(Value)]
struct P7(
    #[route(P0)] Nil,
    #[route(P1)] Nil,
    #[route(P2)] Nil,
    #[route(P3)] Channel,
    #[route(P4)] Nil,
    #[route(P5)] Channel,
    #[route(P6)] Channel,
);

#[derive(Message)]
struct Value(Complex32);

#[session]
#[rustfmt::skip]
type Process<R1, R2, R3> = Send<R1, Value, Receive<R1, Value, Send<R2, Value, Receive<R2, Value, Send<R3, Value, Receive<R3, Value, End>>>>>>;

async fn process<R, R1, R2, R3>(role: &mut R, i: usize, input: Complex32) -> Result<Complex32>
where
    R: Route<R1, Route = Channel>
        + Route<R2, Route = Channel>
        + Route<R3, Route = Channel>
        + Role<Message = Value>,
{
    let angle = |k| (-2.0 * PI * Complex::i() * k as f32 / 8.0).exp();
    let angles = [angle(0), angle(2 * (i % 2)), angle(i % 4)];

    let op = |j: usize| match i & (1 << j) {
        0 => |x, y, angle| x + angle * y,
        _ => |x, y, angle| y - angle * x,
    };
    let ops = [op(0), op(1), op(2)];

    try_session(role, |s: Process<'_, _, R1, R2, R3>| async {
        let x = input;

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = ops[0](x, y, angles[0]);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = ops[1](x, y, angles[1]);

        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        let x = ops[2](x, y, angles[2]);

        Ok((x, s))
    })
    .await
}

struct Vector<'a>(&'a [Complex32]);

impl<'a> Display for Vector<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;

        if !self.0.is_empty() {
            writeln!(f)?;
            for x in self.0 {
                writeln!(f, "    {:.3},", x)?;
            }
        }

        write!(f, "]")
    }
}

fn main() {
    let Roles(mut p0, mut p1, mut p2, mut p3, mut p4, mut p5, mut p6, mut p7) = Roles::default();

    let input = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let input = input.iter().map(Complex::from).collect::<Vec<_>>();
    println!("input = {}", Vector(&input));

    let (o0, o1, o2, o3, o4, o5, o6, o7) = executor::block_on(async {
        try_join!(
            process::<P0, P1, P2, P4>(&mut p0, 0, input[0]),
            process::<P1, P0, P3, P5>(&mut p1, 1, input[4]),
            process::<P2, P3, P0, P6>(&mut p2, 2, input[2]),
            process::<P3, P2, P1, P7>(&mut p3, 3, input[6]),
            process::<P4, P5, P6, P0>(&mut p4, 4, input[1]),
            process::<P5, P4, P7, P1>(&mut p5, 5, input[5]),
            process::<P6, P7, P4, P2>(&mut p6, 6, input[3]),
            process::<P7, P6, P5, P3>(&mut p7, 7, input[7]),
        )
        .unwrap()
    });

    let output = [o0, o1, o2, o3, o4, o5, o6, o7];
    println!("output = {}", Vector(&output));
}
