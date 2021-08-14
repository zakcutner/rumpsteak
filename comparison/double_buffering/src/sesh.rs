use sesh::{close, fork_with_thread_id, recv, send, End, Recv, Send, Session};
use std::{error::Error, result, sync::Arc, thread};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

struct Ready;
struct Value(Vec<i32>);

type Source = Recv<Ready, Send<Value, Recv<Ready, Send<Value, End>>>>;

type Sink = Send<Ready, Recv<Value, Send<Ready, Recv<Value, End>>>>;

fn source(s: Source, values: &[i32]) -> Result<()> {
    let half = values.len() / 2;

    let (Ready, s) = recv(s)?;
    let s = send(Value(values[..half].to_vec()), s);

    let (Ready, s) = recv(s)?;
    let s = send(Value(values[half..].to_vec()), s);

    close(s)?;
    Ok(())
}

fn kernel(s: <Source as Session>::Dual, t: <Sink as Session>::Dual) -> Result<()> {
    let s = send(Ready, s);
    let (Value(values), s) = recv(s)?;
    let (Ready, t) = recv(t)?;
    let t = send(Value(values), t);

    let s = send(Ready, s);
    let (Value(values), s) = recv(s)?;
    let (Ready, t) = recv(t)?;
    let t = send(Value(values), t);

    close(s)?;
    close(t)?;
    Ok(())
}

fn sink(s: Sink) -> Result<Vec<i32>> {
    let mut output = Vec::new();

    let s = send(Ready, s);
    let (Value(values), s) = recv(s)?;
    output.extend(values);

    let s = send(Ready, s);
    let (Value(values), s) = recv(s)?;
    output.extend(values);

    close(s)?;
    Ok(output)
}

pub fn run(input: Arc<[i32]>) {
    let (source, s) = fork_with_thread_id({
        let input = input.clone();
        move |s| source(s, &input)
    });

    let (sink, t) = fork_with_thread_id(move |s| {
        let output = sink(s)?;
        assert_eq!(input.as_ref(), output.as_slice());
        Ok(())
    });

    let kernel = thread::spawn(move || kernel(s, t).unwrap());

    source.join().unwrap();
    kernel.join().unwrap();
    sink.join().unwrap();
}
