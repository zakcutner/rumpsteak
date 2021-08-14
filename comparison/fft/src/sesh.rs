use num_complex::Complex32;
use paste::paste;
use sesh::{close, fork_with_thread_id, recv, send, End, Recv, Send};
use std::{error::Error, marker, result, sync::Arc, thread};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

type Value = Arc<[Complex32]>;

type Butterfly = Send<Value, Recv<Value, End>>;
type ButterflyR = Recv<Value, Send<Value, End>>;

fn butterfly_0_4(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_0_2(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_0_1(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_1_5(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_1_3(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_1_0(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_2_6(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_2_0(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_2_3(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_3_7(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_3_1(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    close(s)?;
    Ok(x)
}

fn butterfly_3_2(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_4_0(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_4_6(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_4_5(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_5_1(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_5_7(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| crate::rotate_45(x + y));

    close(s)?;
    Ok(x)
}

fn butterfly_5_4(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_6_2(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    close(s)?;
    Ok(x)
}

fn butterfly_6_4(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

fn butterfly_6_7(s: Butterfly, x: Value) -> Result<Value> {
    let s = send(x.clone(), s);
    let (y, s) = recv(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close(s)?;
    Ok(x)
}

fn butterfly_7_3(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    close(s)?;
    Ok(x)
}

fn butterfly_7_5(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_135(y - x));

    close(s)?;
    Ok(x)
}

fn butterfly_7_6(s: ButterflyR, x: Value) -> Result<Value> {
    let (y, s) = recv(s)?;
    let s = send(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close(s)?;
    Ok(x)
}

struct ComplexCell(*mut Value);

unsafe impl marker::Send for ComplexCell {}

macro_rules! fork {
    ($vector:ident, $(($left:literal, $right:literal)),*) => {
        $(paste! {
            let v = ComplexCell(&mut $vector[$left]);
            let ([<t_ $left _ $right>], s) = fork_with_thread_id(move |s| unsafe {
                Ok(*v.0 = [<butterfly_ $left _ $right>](s, (*v.0).clone())?)
            });

            let v = ComplexCell(&mut $vector[$right]);
            let [<t_ $right _ $left>] = thread::spawn(move || unsafe {
                *v.0 = [<butterfly_ $right _ $left>](s, (*v.0).clone()).unwrap();
            });
        })*

        $(paste! {
            [<t_ $left _ $right>].join().unwrap();
            [<t_ $right _ $left>].join().unwrap();
        })*
    };
}

pub fn run(input: &[Value; 8]) -> [Value; 8] {
    let mut vector = input.clone();

    fork!(vector, (0, 4), (1, 5), (2, 6), (3, 7));
    fork!(vector, (0, 2), (1, 3), (4, 6), (5, 7));
    fork!(vector, (0, 1), (2, 3), (4, 5), (6, 7));

    [
        vector[0].clone(),
        vector[4].clone(),
        vector[2].clone(),
        vector[6].clone(),
        vector[1].clone(),
        vector[5].clone(),
        vector[3].clone(),
        vector[7].clone(),
    ]
}
