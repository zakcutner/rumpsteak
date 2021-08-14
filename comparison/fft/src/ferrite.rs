use ferrite_session::prelude::*;
use futures::join;
use num_complex::Complex32;
use paste::paste;
use std::sync::Arc;

type Value = Arc<[Complex32]>;

type Butterfly = ReceiveChannel<ButterflyR, SendValue<(Value, Value), End>>;
type ButterflyR = ReceiveValue<Value, SendValue<Value, SendValue<Value, End>>>;

fn butterfly_0_4(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_0_2(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_0_1(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_1_5(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_1_3(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_1_0(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_2_6(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_2_0(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_2_3(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_3_7(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_3_1(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));
            send_value(x, terminate())
        })
    })
}

fn butterfly_3_2(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_4_0(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_4_6(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_4_5(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_5_1(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_5_7(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| crate::rotate_45(x + y));
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_5_4(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_6_2(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));
            send_value(x, terminate())
        })
    })
}

fn butterfly_6_4(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

fn butterfly_6_7(x: Value) -> Session<Butterfly> {
    receive_channel(|c| {
        send_value_to(c, x.clone(), {
            receive_value_from(c, move |y| {
                let x = crate::zip_with(x, y, |x, y| x + y);
                receive_value_from(c, move |y| send_value((x, y), wait(c, terminate())))
            })
        })
    })
}

fn butterfly_7_3(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));
            send_value(x, terminate())
        })
    })
}

fn butterfly_7_5(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| crate::rotate_135(y - x));
            send_value(x, terminate())
        })
    })
}

fn butterfly_7_6(x: Value) -> Session<ButterflyR> {
    receive_value(move |y| {
        send_value(x.clone(), {
            let x = crate::zip_with(x, y, |x, y| y - x);
            send_value(x, terminate())
        })
    })
}

macro_rules! fork {
    ($vector:ident, $(($left:literal, $right:literal)),*) => {
        let ($(paste! { ([<v $left>], [<v $right>]) }),*) = join!($(paste! {
            run_session_with_result(apply_channel(
                [<butterfly_ $left _ $right>]($vector[$left].clone()),
                [<butterfly_ $right _ $left>]($vector[$right].clone()),
            ))
        }),*);

        $(
            $vector[$left] = paste! { [<v $left>] };
            $vector[$right] = paste! { [<v $right>] };
        )*
    };
}

pub async fn run(input: &[Arc<[Complex32]>; 8]) -> [Arc<[Complex32]>; 8] {
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
