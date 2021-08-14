#![allow(clippy::nonstandard_macro_braces)]

use mpstthree::{
    binary::struct_trait::{End, Recv, Send},
    bundle_struct_fork_close_multi, create_multiple_normal_role, create_recv_mpst_session_bundle,
    create_send_mpst_session_bundle,
    role::end::RoleEnd,
};
use std::{error::Error, result, sync::Arc};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

bundle_struct_fork_close_multi!(close_mpst, fork_mpst, MeshedChannelsThree, 3);

create_multiple_normal_role!(
    RoleS, RoleSDual |
    RoleK, RoleKDual |
    RoleT, RoleTDual |
);

create_send_mpst_session_bundle!(
    send_mpst_s_to_k, RoleK, 1 | =>
    RoleS, MeshedChannelsThree, 3
);

create_send_mpst_session_bundle!(
    send_mpst_k_to_s, RoleS, 1 | =>
    RoleK, MeshedChannelsThree, 3
);

create_send_mpst_session_bundle!(
    send_mpst_k_to_t, RoleT, 2 | =>
    RoleK, MeshedChannelsThree, 3
);

create_send_mpst_session_bundle!(
    send_mpst_t_to_k, RoleK, 2 | =>
    RoleT, MeshedChannelsThree, 3
);

create_recv_mpst_session_bundle!(
    recv_mpst_s_from_k, RoleK, 1 | =>
    RoleS, MeshedChannelsThree, 3
);

create_recv_mpst_session_bundle!(
    recv_mpst_k_from_s, RoleS, 1 | =>
    RoleK, MeshedChannelsThree, 3
);

create_recv_mpst_session_bundle!(
    recv_mpst_k_from_t, RoleT, 2 | =>
    RoleK, MeshedChannelsThree, 3
);

create_recv_mpst_session_bundle!(
    recv_mpst_t_from_k, RoleK, 2 | =>
    RoleT, MeshedChannelsThree, 3
);

struct Ready;
struct Value(Vec<i32>);

type NameS = RoleS<RoleEnd>;
type NameK = RoleK<RoleEnd>;
type NameT = RoleT<RoleEnd>;

type EndpointS = MeshedChannelsThree<
    Recv<Ready, Send<Value, Recv<Ready, Send<Value, End>>>>,
    End,
    RoleK<RoleK<RoleK<RoleK<RoleEnd>>>>,
    NameS,
>;

type EndpointK = MeshedChannelsThree<
    Send<Ready, Recv<Value, Send<Ready, Recv<Value, End>>>>,
    Recv<Ready, Send<Value, Recv<Ready, Send<Value, End>>>>,
    RoleS<RoleS<RoleT<RoleT<RoleS<RoleS<RoleT<RoleT<RoleEnd>>>>>>>>,
    NameK,
>;

type EndpointT = MeshedChannelsThree<
    End,
    Send<Ready, Recv<Value, Send<Ready, Recv<Value, End>>>>,
    RoleK<RoleK<RoleK<RoleK<RoleEnd>>>>,
    NameT,
>;

fn source(s: EndpointS, values: &[i32]) -> Result<()> {
    let half = values.len() / 2;

    let (Ready, s) = recv_mpst_s_from_k(s)?;
    let s = send_mpst_s_to_k(Value(values[..half].to_vec()), s);

    let (Ready, s) = recv_mpst_s_from_k(s)?;
    let s = send_mpst_s_to_k(Value(values[half..].to_vec()), s);

    close_mpst(s)?;
    Ok(())
}

fn kernel(s: EndpointK) -> Result<()> {
    let s = send_mpst_k_to_s(Ready, s);
    let (Value(values), s) = recv_mpst_k_from_s(s)?;
    let (Ready, s) = recv_mpst_k_from_t(s)?;
    let s = send_mpst_k_to_t(Value(values), s);

    let s = send_mpst_k_to_s(Ready, s);
    let (Value(values), s) = recv_mpst_k_from_s(s)?;
    let (Ready, s) = recv_mpst_k_from_t(s)?;
    let s = send_mpst_k_to_t(Value(values), s);

    close_mpst(s)?;
    Ok(())
}

fn sink(s: EndpointT) -> Result<Vec<i32>> {
    let mut output = Vec::new();

    let s = send_mpst_t_to_k(Ready, s);
    let (Value(values), s) = recv_mpst_t_from_k(s)?;
    output.extend(values);

    let s = send_mpst_t_to_k(Ready, s);
    let (Value(values), s) = recv_mpst_t_from_k(s)?;
    output.extend(values);

    close_mpst(s)?;
    Ok(output)
}

pub fn run(input: Arc<[i32]>) {
    let (source, kernel, sink) = fork_mpst(
        {
            let input = input.clone();
            move |s| source(s, &input)
        },
        kernel,
        move |s| {
            let output = sink(s)?;
            assert_eq!(input.as_ref(), output.as_slice());
            Ok(())
        },
    );

    source.join().unwrap();
    kernel.join().unwrap();
    sink.join().unwrap();
}
