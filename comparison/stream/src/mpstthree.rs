#![allow(clippy::nonstandard_macro_braces)]

use mpstthree::{
    binary::struct_trait::{End, Recv, Send, Session},
    bundle_struct_fork_close_multi, choose_mpst_multi_to_all, create_multiple_normal_role,
    create_recv_mpst_session_bundle, create_send_mpst_session_bundle, offer_mpst,
    role::{broadcast::RoleBroadcast, end::RoleEnd},
};
use std::{error::Error, result, sync::Arc};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

bundle_struct_fork_close_multi!(close_mpst, fork_mpst, MeshedChannelsTwo, 2);

create_multiple_normal_role!(
    RoleS, RoleSDual |
    RoleT, RoleTDual |
);

create_send_mpst_session_bundle!(
    send_mpst_s_to_t, RoleT, 1 | =>
    RoleS, MeshedChannelsTwo, 2
);

create_send_mpst_session_bundle!(
    send_mpst_t_to_s, RoleS, 1 | =>
    RoleT, MeshedChannelsTwo, 2
);

create_recv_mpst_session_bundle!(
    recv_mpst_s_from_t, RoleT, 1 | =>
    RoleS, MeshedChannelsTwo, 2
);

create_recv_mpst_session_bundle!(
    recv_mpst_t_from_s, RoleS, 1 | =>
    RoleT, MeshedChannelsTwo, 2
);

struct Ready;
struct Value(i32);

type NameS = RoleS<RoleEnd>;
type NameT = RoleT<RoleEnd>;

type Source = <Sink as Session>::Dual;

type Sink = Send<Ready, Recv<Choice, End>>;

enum Choice {
    Value(MeshedChannelsTwo<Recv<Value, Sink>, RoleS<RoleS<RoleS<RoleEnd>>>, NameT>),
    Stop(MeshedChannelsTwo<End, RoleEnd, NameT>),
}

type EndpointS = MeshedChannelsTwo<Source, RoleT<RoleBroadcast>, NameS>;
type EndpointT = MeshedChannelsTwo<Sink, RoleS<RoleS<RoleEnd>>, NameT>;

fn source(s: EndpointS, values: &[i32]) -> Result<()> {
    let (Ready, mut s) = recv_mpst_s_from_t(s)?;
    for &value in values {
        s = {
            let s = choose_mpst_multi_to_all!(
                s,
                Choice::Value, =>
                RoleT, =>
                RoleS,
                MeshedChannelsTwo,
                1
            );

            let s = send_mpst_s_to_t(Value(value), s);
            let (Ready, s) = recv_mpst_s_from_t(s)?;
            s
        };
    }

    let s = choose_mpst_multi_to_all!(
        s,
        Choice::Stop, =>
        RoleT, =>
        RoleS,
        MeshedChannelsTwo,
        1
    );

    close_mpst(s)?;
    Ok(())
}

fn sink_inner(s: EndpointT, output: &mut Vec<i32>) -> Result<()> {
    let s = send_mpst_t_to_s(Ready, s);
    offer_mpst!(s, recv_mpst_t_from_s, {
        Choice::Value(s) => {
            let (Value(value), s) = recv_mpst_t_from_s(s)?;
            output.push(value);
            sink_inner(s, output)
        },
        Choice::Stop(s) => close_mpst(s),
    })?;

    Ok(())
}

fn sink(s: EndpointT) -> Result<Vec<i32>> {
    let mut output = Vec::new();
    sink_inner(s, &mut output)?;
    Ok(output)
}

pub fn run(input: Arc<[i32]>) {
    let (source, sink) = fork_mpst(
        {
            let input = input.clone();
            move |s| source(s, &input)
        },
        move |s| {
            let output = sink(s)?;
            assert_eq!(input.as_ref(), output.as_slice());
            Ok(())
        },
    );

    source.join().unwrap();
    sink.join().unwrap();
}
