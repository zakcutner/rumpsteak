use sesh::{
    cancel, choose, close, fork_with_thread_id, offer, recv, send, End, Recv, Send, Session,
};
use std::{error::Error, result, sync::Arc};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

struct Ready;
struct Value(i32);

type Source = <Sink as Session>::Dual;

type Sink = Send<Ready, Recv<Choice, End>>;

enum Choice {
    Value(Recv<Value, Sink>),
    Stop(End),
}

fn source(s: Source, values: &[i32]) -> Result<()> {
    let (Ready, mut s) = recv(s)?;
    for &value in values {
        s = {
            let s = choose!(Choice::Value, s);
            let s = send(Value(value), s);
            let (Ready, s) = recv(s)?;
            s
        };
    }

    let s = choose!(Choice::Stop, s);
    close(s)?;

    Ok(())
}

fn sink_inner(s: Sink, output: &mut Vec<i32>) -> Result<()> {
    let s = send(Ready, s);
    offer!(s, {
        Choice::Value(s) => {
            let (Value(value), s) = recv(s)?;
            output.push(value);
            sink_inner(s, output)
        },
        Choice::Stop(s) => close(s),
    })?;

    Ok(())
}

fn sink(s: Sink) -> Result<Vec<i32>> {
    let mut output = Vec::new();
    sink_inner(s, &mut output)?;
    Ok(output)
}

pub fn run(input: Arc<[i32]>) {
    let (thread, s) = fork_with_thread_id({
        let input = input.clone();
        move |s| source(s, &input)
    });

    let output = sink(s).unwrap();
    thread.join().unwrap();

    assert_eq!(input.as_ref(), output.as_slice());
}
