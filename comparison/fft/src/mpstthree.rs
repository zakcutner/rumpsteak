#![allow(clippy::nonstandard_macro_braces)]

use mpstthree::{
    binary::struct_trait::{End, Recv, Send},
    bundle_struct_fork_close_multi, create_multiple_normal_role, create_recv_mpst_session_bundle,
    create_send_mpst_session_bundle,
    role::end::RoleEnd,
};
use num_complex::Complex32;
use std::{error::Error, marker, result, sync::Arc};

type Result<T, E = Box<dyn Error>> = result::Result<T, E>;

bundle_struct_fork_close_multi!(close_mpst, fork_mpst, MeshedChannels, 8);

create_multiple_normal_role!(
    Role0, Role0Dual |
    Role1, Role1Dual |
    Role2, Role2Dual |
    Role3, Role3Dual |
    Role4, Role4Dual |
    Role5, Role5Dual |
    Role6, Role6Dual |
    Role7, Role7Dual |
);

create_send_mpst_session_bundle!(
    send_mpst_0_to_1, Role1, 1 |
    send_mpst_0_to_2, Role2, 2 |
    send_mpst_0_to_4, Role4, 4 | =>
    Role0, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_0_from_1, Role1, 1 |
    recv_mpst_0_from_2, Role2, 2 |
    recv_mpst_0_from_4, Role4, 4 | =>
    Role0, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_1_to_0, Role0, 1 |
    send_mpst_1_to_3, Role3, 3 |
    send_mpst_1_to_5, Role5, 5 | =>
    Role1, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_1_from_0, Role0, 1 |
    recv_mpst_1_from_3, Role3, 3 |
    recv_mpst_1_from_5, Role5, 5 | =>
    Role1, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_2_to_0, Role0, 1 |
    send_mpst_2_to_3, Role3, 3 |
    send_mpst_2_to_6, Role6, 6 | =>
    Role2, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_2_from_0, Role0, 1 |
    recv_mpst_2_from_3, Role3, 3 |
    recv_mpst_2_from_6, Role6, 6 | =>
    Role2, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_3_to_1, Role1, 2 |
    send_mpst_3_to_2, Role2, 3 |
    send_mpst_3_to_7, Role7, 7 | =>
    Role3, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_3_from_1, Role1, 2 |
    recv_mpst_3_from_2, Role2, 3 |
    recv_mpst_3_from_7, Role7, 7 | =>
    Role3, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_4_to_0, Role0, 1 |
    send_mpst_4_to_5, Role5, 5 |
    send_mpst_4_to_6, Role6, 6 | =>
    Role4, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_4_from_0, Role0, 1 |
    recv_mpst_4_from_5, Role5, 5 |
    recv_mpst_4_from_6, Role6, 6 | =>
    Role4, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_5_to_1, Role1, 2 |
    send_mpst_5_to_4, Role4, 5 |
    send_mpst_5_to_7, Role7, 7 | =>
    Role5, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_5_from_1, Role1, 2 |
    recv_mpst_5_from_4, Role4, 5 |
    recv_mpst_5_from_7, Role7, 7 | =>
    Role5, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_6_to_2, Role2, 3 |
    send_mpst_6_to_4, Role4, 5 |
    send_mpst_6_to_7, Role7, 7 | =>
    Role6, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_6_from_2, Role2, 3 |
    recv_mpst_6_from_4, Role4, 5 |
    recv_mpst_6_from_7, Role7, 7 | =>
    Role6, MeshedChannels, 8
);

create_send_mpst_session_bundle!(
    send_mpst_7_to_3, Role3, 4 |
    send_mpst_7_to_5, Role5, 6 |
    send_mpst_7_to_6, Role6, 7 | =>
    Role7, MeshedChannels, 8
);

create_recv_mpst_session_bundle!(
    recv_mpst_7_from_3, Role3, 4 |
    recv_mpst_7_from_5, Role5, 6 |
    recv_mpst_7_from_6, Role6, 7 | =>
    Role7, MeshedChannels, 8
);

type Name0 = Role0<RoleEnd>;
type Name1 = Role1<RoleEnd>;
type Name2 = Role2<RoleEnd>;
type Name3 = Role3<RoleEnd>;
type Name4 = Role4<RoleEnd>;
type Name5 = Role5<RoleEnd>;
type Name6 = Role6<RoleEnd>;
type Name7 = Role7<RoleEnd>;

type DRole0<R> = Role0<Role0<R>>;
type DRole1<R> = Role1<Role1<R>>;
type DRole2<R> = Role2<Role2<R>>;
type DRole3<R> = Role3<Role3<R>>;
type DRole4<R> = Role4<Role4<R>>;
type DRole5<R> = Role5<Role5<R>>;
type DRole6<R> = Role6<Role6<R>>;
type DRole7<R> = Role7<Role7<R>>;

type Butterfly = Send<Complex32, Recv<Complex32, End>>;
type ButterflyR = Recv<Complex32, Send<Complex32, End>>;

#[rustfmt::skip]
type Butterfly0 = MeshedChannels<Butterfly, Butterfly, End, Butterfly, End, End, End, DRole4<DRole2<DRole1<RoleEnd>>>, Name0>;

fn butterfly_0(s: Butterfly0, x: Complex32) -> Result<Complex32> {
    let s = send_mpst_0_to_4(x, s);
    let (y, s) = recv_mpst_0_from_4(s)?;
    let x = x + y;

    let s = send_mpst_0_to_2(x, s);
    let (y, s) = recv_mpst_0_from_2(s)?;
    let x = x + y;

    let s = send_mpst_0_to_1(x, s);
    let (y, s) = recv_mpst_0_from_1(s)?;
    let x = x + y;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly1 = MeshedChannels<ButterflyR, End, Butterfly, End, Butterfly, End, End, DRole5<DRole3<DRole0<RoleEnd>>>, Name1>;

fn butterfly_1(s: Butterfly1, x: Complex32) -> Result<Complex32> {
    let s = send_mpst_1_to_5(x, s);
    let (y, s) = recv_mpst_1_from_5(s)?;
    let x = x + y;

    let s = send_mpst_1_to_3(x, s);
    let (y, s) = recv_mpst_1_from_3(s)?;
    let x = x + y;

    let (y, s) = recv_mpst_1_from_0(s)?;
    let s = send_mpst_1_to_0(x, s);
    let x = y - x;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly2 = MeshedChannels<ButterflyR, End, Butterfly, End, End, Butterfly, End, DRole6<DRole0<DRole3<RoleEnd>>>, Name2>;

fn butterfly_2(s: Butterfly2, x: Complex32) -> Result<Complex32> {
    let s = send_mpst_2_to_6(x, s);
    let (y, s) = recv_mpst_2_from_6(s)?;
    let x = x + y;

    let (y, s) = recv_mpst_2_from_0(s)?;
    let s = send_mpst_2_to_0(x, s);
    let x = y - x;

    let s = send_mpst_2_to_3(x, s);
    let (y, s) = recv_mpst_2_from_3(s)?;
    let x = x + y;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly3 = MeshedChannels<End, ButterflyR, ButterflyR, End, End, End, Butterfly, DRole7<DRole1<DRole2<RoleEnd>>>, Name3>;

fn butterfly_3(s: Butterfly3, x: Complex32) -> Result<Complex32> {
    let s = send_mpst_3_to_7(x, s);
    let (y, s) = recv_mpst_3_from_7(s)?;
    let x = x + y;

    let (y, s) = recv_mpst_3_from_1(s)?;
    let s = send_mpst_3_to_1(x, s);
    let x = crate::rotate_90(y - x);

    let (y, s) = recv_mpst_3_from_2(s)?;
    let s = send_mpst_3_to_2(x, s);
    let x = y - x;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly4 = MeshedChannels<ButterflyR, End, End, End, Butterfly, Butterfly, End, DRole0<DRole6<DRole5<RoleEnd>>>, Name4>;

fn butterfly_4(s: Butterfly4, x: Complex32) -> Result<Complex32> {
    let (y, s) = recv_mpst_4_from_0(s)?;
    let s = send_mpst_4_to_0(x, s);
    let x = y - x;

    let s = send_mpst_4_to_6(x, s);
    let (y, s) = recv_mpst_4_from_6(s)?;
    let x = x + y;

    let s = send_mpst_4_to_5(x, s);
    let (y, s) = recv_mpst_4_from_5(s)?;
    let x = x + y;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly5 = MeshedChannels<End, ButterflyR, End, End, ButterflyR, End, Butterfly, DRole1<DRole7<DRole4<RoleEnd>>>, Name5>;

fn butterfly_5(s: Butterfly5, x: Complex32) -> Result<Complex32> {
    let (y, s) = recv_mpst_5_from_1(s)?;
    let s = send_mpst_5_to_1(x, s);
    let x = y - x;

    let s = send_mpst_5_to_7(x, s);
    let (y, s) = recv_mpst_5_from_7(s)?;
    let x = crate::rotate_45(x + y);

    let (y, s) = recv_mpst_5_from_4(s)?;
    let s = send_mpst_5_to_4(x, s);
    let x = y - x;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly6 = MeshedChannels<End, End, ButterflyR, End, ButterflyR, End, Butterfly, DRole2<DRole4<DRole7<RoleEnd>>>, Name6>;

fn butterfly_6(s: Butterfly6, x: Complex32) -> Result<Complex32> {
    let (y, s) = recv_mpst_6_from_2(s)?;
    let s = send_mpst_6_to_2(x, s);
    let x = crate::rotate_90(y - x);

    let (y, s) = recv_mpst_6_from_4(s)?;
    let s = send_mpst_6_to_4(x, s);
    let x = y - x;

    let s = send_mpst_6_to_7(x, s);
    let (y, s) = recv_mpst_6_from_7(s)?;
    let x = x + y;

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly7 = MeshedChannels<End, End, End, ButterflyR, End, ButterflyR, ButterflyR, DRole3<DRole5<DRole6<RoleEnd>>>, Name7>;

fn butterfly_7(s: Butterfly7, x: Complex32) -> Result<Complex32> {
    let (y, s) = recv_mpst_7_from_3(s)?;
    let s = send_mpst_7_to_3(x, s);
    let x = crate::rotate_90(y - x);

    let (y, s) = recv_mpst_7_from_5(s)?;
    let s = send_mpst_7_to_5(x, s);
    let x = crate::rotate_135(y - x);

    let (y, s) = recv_mpst_7_from_6(s)?;
    let s = send_mpst_7_to_6(x, s);
    let x = y - x;

    close_mpst(s)?;
    Ok(x)
}

struct ComplexCell(*mut Complex32);

unsafe impl marker::Send for ComplexCell {}

pub fn run(input: Arc<[Complex32]>) -> Vec<Complex32> {
    let mut threads = Vec::with_capacity(input.len() / 8);
    let mut output = vec![Default::default(); input.len()];

    for i in (0..input.len()).step_by(8) {
        let (r0, r1, r2, r3, r4, r5, r6, r7) = fork_mpst(
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_0(s, input[i])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 4] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_1(s, input[i + 1])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 2] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_2(s, input[i + 2])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 6] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_3(s, input[i + 3])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 1] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_4(s, input[i + 4])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 5] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_5(s, input[i + 5])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 3] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_6(s, input[i + 6])? };
                    Ok(())
                }
            },
            {
                let (input, output) = (input.clone(), ComplexCell(&mut output[i + 7] as *mut _));
                move |s| {
                    unsafe { *output.0 = butterfly_7(s, input[i + 7])? };
                    Ok(())
                }
            },
        );

        threads.extend([r0, r1, r2, r3, r4, r5, r6, r7]);
    }

    for thread in threads {
        thread.join().unwrap();
    }

    output
}
