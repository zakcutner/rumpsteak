
async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: AdderC<'_, _>| async {
        let s = s.send(Lhs(10)).await?;
        let s = s.send(Rhs(10)).await?;
        let (Res(v), s) = s.receive().await?;

        println!("{:?}", v);
        return Ok(((), s))
    })
    .await
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: AdderS<'_, _>| async {
        let (Lhs(x), s) = s.receive().await?;
        let (Rhs(y), s) = s.receive().await?;
        let s = s.send(Res(x + y)).await?;
        return Ok(((), s))
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(s(&mut roles.s), c(&mut roles.c)).unwrap();
    });
}
