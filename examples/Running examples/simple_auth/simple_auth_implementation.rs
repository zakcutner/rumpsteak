async fn C(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: AuthC<'_, _>| async {
        let mut s = s.send(SetPw(10000000)).await?;
        let mut cur_attempt = 0;
        loop {
            let s_send = s.send(Password(cur_attempt)).await?;
            cur_attempt += 1;
            match s_send.branch().await? {
                AuthC3::Success(_, s_bra) => {
                    println!("Success ({} attempts)", cur_attempt);
                    return Ok(((), s_bra));
                }
                AuthC3::Failure(_, s_bra) => {
                    //println!("Failure");
                    let (_, s_retres) = s_bra.receive().await?;
                    s = s_retres.send(RetRes(0)).await?;
                }
            }
        }
    })
    .await
}
async fn S(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: AuthS<'_, _>| async {
        let (SetPw(p), mut s) = s.receive().await?;
        let password = p;
        loop {
            let (Password(n), s_rec) = s.receive().await?;
            if n == password {
                let s_end = s_rec.select(Success(0)).await?;
                return Ok(((), s_end));
            } else {
                let s_fail = s_rec.select(Failure(-1)).await?;
                let s_retx = s_fail.send(RetX(0)).await?;
                (_, s) = s_retx.receive().await?;
            }
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(C(&mut roles.c), S(&mut roles.s)).unwrap();
    });
}
