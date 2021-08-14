use ferrite_session::prelude::*;
use futures::lock::Mutex;
use std::sync::Arc;

struct Ready;
struct Value(Vec<i32>);

type Source = ReceiveValue<Ready, SendValue<Value, End>>;

type Kernel =
    ReceiveChannel<Source, ReceiveChannel<Sink, ReceiveChannel<Source, ReceiveChannel<Sink, End>>>>;

type Sink = SendValue<Ready, ReceiveValue<Value, End>>;

fn source(values: Arc<[i32]>) -> (Session<Source>, Session<Source>) {
    let half = values.len() / 2;

    let right = {
        let values = values.clone();
        receive_value(move |Ready| send_value(Value(values[half..].to_vec()), terminate()))
    };

    let left = receive_value(move |Ready| send_value(Value(values[..half].to_vec()), terminate()));

    (left, right)
}

fn kernel() -> Session<Kernel> {
    receive_channel(move |c| {
        send_value_to(
            c,
            Ready,
            receive_value_from(c, move |Value(values)| {
                wait(
                    c,
                    receive_channel(move |c| {
                        receive_value_from(c, move |Ready| {
                            send_value_to(
                                c,
                                Value(values),
                                wait(
                                    c,
                                    receive_channel(move |c| {
                                        send_value_to(
                                            c,
                                            Ready,
                                            receive_value_from(c, move |Value(values)| {
                                                wait(
                                                    c,
                                                    receive_channel(move |c| {
                                                        receive_value_from(c, move |Ready| {
                                                            send_value_to(
                                                                c,
                                                                Value(values),
                                                                wait(c, terminate()),
                                                            )
                                                        })
                                                    }),
                                                )
                                            }),
                                        )
                                    }),
                                ),
                            )
                        })
                    }),
                )
            }),
        )
    })
}

fn sink(output: Arc<Mutex<Vec<i32>>>) -> (Session<Sink>, Session<Sink>) {
    let right = {
        let output = output.clone();
        send_value(
            Ready,
            receive_value(move |Value(values)| {
                step(async move {
                    output.lock().await.extend(values);
                    terminate()
                })
            }),
        )
    };

    let left = send_value(
        Ready,
        receive_value(move |Value(values)| {
            step(async move {
                output.lock().await.extend(values);
                terminate()
            })
        }),
    );

    (left, right)
}

pub async fn run(input: Arc<[i32]>) {
    let output = Arc::new(Mutex::new(Vec::new()));

    let (left_s, right_s) = source(input.clone());
    let (left_t, right_t) = sink(output.clone());

    let k = apply_channel(kernel(), left_s);
    let k = apply_channel(k, left_t);
    let k = apply_channel(k, right_s);
    run_session(apply_channel(k, right_t)).await;

    assert_eq!(input.as_ref(), output.lock().await.as_slice());
}
