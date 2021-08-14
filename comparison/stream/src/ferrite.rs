use ferrite_session::{either::*, prelude::*};
use futures::lock::Mutex;
use std::sync::Arc;

struct Ready;
struct Value(i32);

type Source = ReceiveChannel<Sink, End>;

type Sink = Rec<SendValue<Ready, ExternalChoice<Either<ReceiveValue<Value, Z>, End>>>>;

fn source_inner(values: Arc<[i32]>, index: usize) -> Session<Source> {
    receive_channel(|c| {
        unfix_session(
            c,
            receive_value_from(c, move |Ready| {
                if let Some(&value) = values.get(index) {
                    return choose!(
                        c,
                        Left,
                        send_value_to(
                            c,
                            Value(value),
                            include_session(source_inner(values, index + 1), |consume| {
                                send_channel_to(consume, c, wait(consume, terminate()))
                            })
                        )
                    );
                }

                choose!(c, Right, wait(c, terminate()))
            }),
        )
    })
}

fn source(values: Arc<[i32]>) -> Session<Source> {
    source_inner(values, 0)
}

fn sink(output: Arc<Mutex<Vec<i32>>>) -> Session<Sink> {
    fix_session(send_value(
        Ready,
        offer_choice!(
            Left => {receive_value(|Value(value)|
                step(async move {
                    output.lock().await.push(value);
                    sink(output)
                })
            )}
            Right => terminate()
        ),
    ))
}

pub async fn run(input: Arc<[i32]>) {
    let output = Arc::new(Mutex::new(Vec::new()));
    run_session(apply_channel(source(input.clone()), sink(output.clone()))).await;
    assert_eq!(input.as_ref(), output.lock().await.as_slice());
}
