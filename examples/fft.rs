use futures::{executor, try_join};
use num_complex::{Complex, Complex32};
use rumpsteak::{
    role::{Nil, Role, Roles, Route, ToFrom},
    try_session, End, Label, Receive, Result, Send,
};
use std::{
    f32::consts::PI,
    fmt::{self, Display, Formatter},
};

#[derive(Roles)]
struct Roles(P0, P1, P2, P3, P4, P5, P6, P7);

#[derive(Role)]
#[message(Message)]
struct P0(
    #[route(P1)] ToFrom<P1>,
    #[route(P2)] ToFrom<P2>,
    #[route(P3)] Nil<P3>,
    #[route(P4)] ToFrom<P4>,
    #[route(P5)] Nil<P5>,
    #[route(P6)] Nil<P6>,
    #[route(P7)] Nil<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P1(
    #[route(P0)] ToFrom<P0>,
    #[route(P2)] Nil<P2>,
    #[route(P3)] ToFrom<P3>,
    #[route(P4)] Nil<P4>,
    #[route(P5)] ToFrom<P5>,
    #[route(P6)] Nil<P6>,
    #[route(P7)] Nil<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P2(
    #[route(P0)] ToFrom<P0>,
    #[route(P1)] Nil<P1>,
    #[route(P3)] ToFrom<P3>,
    #[route(P4)] Nil<P4>,
    #[route(P5)] Nil<P5>,
    #[route(P6)] ToFrom<P6>,
    #[route(P7)] Nil<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P3(
    #[route(P0)] Nil<P0>,
    #[route(P1)] ToFrom<P1>,
    #[route(P2)] ToFrom<P2>,
    #[route(P4)] Nil<P4>,
    #[route(P5)] Nil<P5>,
    #[route(P6)] Nil<P6>,
    #[route(P7)] ToFrom<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P4(
    #[route(P0)] ToFrom<P0>,
    #[route(P1)] Nil<P1>,
    #[route(P2)] Nil<P2>,
    #[route(P3)] Nil<P3>,
    #[route(P5)] ToFrom<P5>,
    #[route(P6)] ToFrom<P6>,
    #[route(P7)] Nil<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P5(
    #[route(P0)] Nil<P0>,
    #[route(P1)] ToFrom<P1>,
    #[route(P2)] Nil<P2>,
    #[route(P3)] Nil<P3>,
    #[route(P4)] ToFrom<P4>,
    #[route(P6)] Nil<P6>,
    #[route(P7)] ToFrom<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P6(
    #[route(P0)] Nil<P0>,
    #[route(P1)] Nil<P1>,
    #[route(P2)] ToFrom<P2>,
    #[route(P3)] Nil<P3>,
    #[route(P4)] ToFrom<P4>,
    #[route(P5)] Nil<P5>,
    #[route(P7)] ToFrom<P7>,
);

#[derive(Role)]
#[message(Message)]
struct P7(
    #[route(P0)] Nil<P0>,
    #[route(P1)] Nil<P1>,
    #[route(P2)] Nil<P2>,
    #[route(P3)] ToFrom<P3>,
    #[route(P4)] Nil<P4>,
    #[route(P5)] ToFrom<P5>,
    #[route(P6)] ToFrom<P6>,
);

#[derive(Label)]
enum Message {
    Value(Value),
}

struct Value(Complex32);

#[rustfmt::skip]
type Process<'q, Q, R, S, T> = Send<'q, Q, R, Value, Receive<'q, Q, R, Value, Send<'q, Q, S, Value, Receive<'q, Q, S, Value, Send<'q, Q, T, Value, Receive<'q, Q, T, Value, End<'q>>>>>>>;

async fn process<Q, R, S, T>(role: &mut Q, i: usize, input: Complex32) -> Result<Complex32>
where
    Q: Route<R, Route = ToFrom<R>>
        + Route<S, Route = ToFrom<S>>
        + Route<T, Route = ToFrom<T>>
        + Role<Message = Message>,
    R: Route<Q, Route = ToFrom<Q>> + Role<Message = Message>,
    S: Route<Q, Route = ToFrom<Q>> + Role<Message = Message>,
    T: Route<Q, Route = ToFrom<Q>> + Role<Message = Message>,
{
    let angle = |k| (-2.0 * PI * Complex::i() * k as f32 / 8.0).exp();
    let angles = [angle(0), angle(2 * (i % 2)), angle(i % 4)];

    let op = |j: usize| match i & (1 << j) {
        0 => |x, y, angle| x + angle * y,
        _ => |x, y, angle| y - angle * x,
    };
    let ops = [op(0), op(1), op(2)];

    try_session(role, |s: Process<'_, Q, R, S, T>| async {
        let x = input;

        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        let x = ops[0](x, y, angles[0]);

        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        let x = ops[1](x, y, angles[1]);

        let s = s.send(Value(x))?;
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

        let mut input = self.0.iter();
        if let Some(x) = input.next() {
            write!(f, "{}", x)?;
            for x in input {
                write!(f, ", {}", x)?;
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
