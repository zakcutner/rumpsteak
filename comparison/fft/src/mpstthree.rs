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

type Butterfly = Send<Arc<[Complex32]>, Recv<Arc<[Complex32]>, End>>;
type ButterflyR = Recv<Arc<[Complex32]>, Send<Arc<[Complex32]>, End>>;

#[rustfmt::skip]
type Butterfly0 = MeshedChannels<Butterfly, Butterfly, End, Butterfly, End, End, End, DRole4<DRole2<DRole1<RoleEnd>>>, Name0>;

fn butterfly_0(s: Butterfly0, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let s = send_mpst_0_to_4(x.clone(), s);
    let (y, s) = recv_mpst_0_from_4(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let s = send_mpst_0_to_2(x.clone(), s);
    let (y, s) = recv_mpst_0_from_2(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let s = send_mpst_0_to_1(x.clone(), s);
    let (y, s) = recv_mpst_0_from_1(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly1 = MeshedChannels<ButterflyR, End, Butterfly, End, Butterfly, End, End, DRole5<DRole3<DRole0<RoleEnd>>>, Name1>;

fn butterfly_1(s: Butterfly1, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let s = send_mpst_1_to_5(x.clone(), s);
    let (y, s) = recv_mpst_1_from_5(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let s = send_mpst_1_to_3(x.clone(), s);
    let (y, s) = recv_mpst_1_from_3(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let (y, s) = recv_mpst_1_from_0(s)?;
    let s = send_mpst_1_to_0(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly2 = MeshedChannels<ButterflyR, End, Butterfly, End, End, Butterfly, End, DRole6<DRole0<DRole3<RoleEnd>>>, Name2>;

fn butterfly_2(s: Butterfly2, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let s = send_mpst_2_to_6(x.clone(), s);
    let (y, s) = recv_mpst_2_from_6(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let (y, s) = recv_mpst_2_from_0(s)?;
    let s = send_mpst_2_to_0(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    let s = send_mpst_2_to_3(x.clone(), s);
    let (y, s) = recv_mpst_2_from_3(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly3 = MeshedChannels<End, ButterflyR, ButterflyR, End, End, End, Butterfly, DRole7<DRole1<DRole2<RoleEnd>>>, Name3>;

fn butterfly_3(s: Butterfly3, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let s = send_mpst_3_to_7(x.clone(), s);
    let (y, s) = recv_mpst_3_from_7(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let (y, s) = recv_mpst_3_from_1(s)?;
    let s = send_mpst_3_to_1(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    let (y, s) = recv_mpst_3_from_2(s)?;
    let s = send_mpst_3_to_2(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly4 = MeshedChannels<ButterflyR, End, End, End, Butterfly, Butterfly, End, DRole0<DRole6<DRole5<RoleEnd>>>, Name4>;

fn butterfly_4(s: Butterfly4, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let (y, s) = recv_mpst_4_from_0(s)?;
    let s = send_mpst_4_to_0(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    let s = send_mpst_4_to_6(x.clone(), s);
    let (y, s) = recv_mpst_4_from_6(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    let s = send_mpst_4_to_5(x.clone(), s);
    let (y, s) = recv_mpst_4_from_5(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly5 = MeshedChannels<End, ButterflyR, End, End, ButterflyR, End, Butterfly, DRole1<DRole7<DRole4<RoleEnd>>>, Name5>;

fn butterfly_5(s: Butterfly5, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let (y, s) = recv_mpst_5_from_1(s)?;
    let s = send_mpst_5_to_1(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    let s = send_mpst_5_to_7(x.clone(), s);
    let (y, s) = recv_mpst_5_from_7(s)?;
    let x = crate::zip_with(x, y, |x, y| crate::rotate_45(x + y));

    let (y, s) = recv_mpst_5_from_4(s)?;
    let s = send_mpst_5_to_4(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly6 = MeshedChannels<End, End, ButterflyR, End, ButterflyR, End, Butterfly, DRole2<DRole4<DRole7<RoleEnd>>>, Name6>;

fn butterfly_6(s: Butterfly6, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let (y, s) = recv_mpst_6_from_2(s)?;
    let s = send_mpst_6_to_2(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    let (y, s) = recv_mpst_6_from_4(s)?;
    let s = send_mpst_6_to_4(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    let s = send_mpst_6_to_7(x.clone(), s);
    let (y, s) = recv_mpst_6_from_7(s)?;
    let x = crate::zip_with(x, y, |x, y| x + y);

    close_mpst(s)?;
    Ok(x)
}

#[rustfmt::skip]
type Butterfly7 = MeshedChannels<End, End, End, ButterflyR, End, ButterflyR, ButterflyR, DRole3<DRole5<DRole6<RoleEnd>>>, Name7>;

fn butterfly_7(s: Butterfly7, x: Arc<[Complex32]>) -> Result<Arc<[Complex32]>> {
    let (y, s) = recv_mpst_7_from_3(s)?;
    let s = send_mpst_7_to_3(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_90(y - x));

    let (y, s) = recv_mpst_7_from_5(s)?;
    let s = send_mpst_7_to_5(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| crate::rotate_135(y - x));

    let (y, s) = recv_mpst_7_from_6(s)?;
    let s = send_mpst_7_to_6(x.clone(), s);
    let x = crate::zip_with(x, y, |x, y| y - x);

    close_mpst(s)?;
    Ok(x)
}

struct ComplexCell(*mut Arc<[Complex32]>);

unsafe impl marker::Send for ComplexCell {}

pub fn run(input: &[Arc<[Complex32]>; 8]) -> [Arc<[Complex32]>; 8] {
    let (i0, i1, i2, i3, i4, i5, i6, i7) = (
        input[0].clone(),
        input[1].clone(),
        input[2].clone(),
        input[3].clone(),
        input[4].clone(),
        input[5].clone(),
        input[6].clone(),
        input[7].clone(),
    );

    let mut output = input.clone();
    let (o0, o1, o2, o3, o4, o5, o6, o7) = (
        ComplexCell(&mut output[0]),
        ComplexCell(&mut output[1]),
        ComplexCell(&mut output[2]),
        ComplexCell(&mut output[3]),
        ComplexCell(&mut output[4]),
        ComplexCell(&mut output[5]),
        ComplexCell(&mut output[6]),
        ComplexCell(&mut output[7]),
    );

    let (r0, r1, r2, r3, r4, r5, r6, r7) = fork_mpst(
        move |s| {
            unsafe { *o0.0 = butterfly_0(s, i0)? };
            Ok(())
        },
        move |s| {
            unsafe { *o4.0 = butterfly_1(s, i1)? };
            Ok(())
        },
        move |s| {
            unsafe { *o2.0 = butterfly_2(s, i2)? };
            Ok(())
        },
        move |s| {
            unsafe { *o6.0 = butterfly_3(s, i3)? };
            Ok(())
        },
        move |s| {
            unsafe { *o1.0 = butterfly_4(s, i4)? };
            Ok(())
        },
        move |s| {
            unsafe { *o5.0 = butterfly_5(s, i5)? };
            Ok(())
        },
        move |s| {
            unsafe { *o3.0 = butterfly_6(s, i6)? };
            Ok(())
        },
        move |s| {
            unsafe { *o7.0 = butterfly_7(s, i7)? };
            Ok(())
        },
    );

    r0.join().unwrap();
    r1.join().unwrap();
    r2.join().unwrap();
    r3.join().unwrap();
    r4.join().unwrap();
    r5.join().unwrap();
    r6.join().unwrap();
    r7.join().unwrap();

    output
}
