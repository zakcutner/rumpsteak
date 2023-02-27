async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: PlusMinusA<'_, _>| async {
        let s = s.send(Secret(10)).await?;
        return Ok(((), s))
    })
    .await
}

async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: PlusMinusB<'_, _>| async {
        let (Secret(n), s) = s.receive().await?;
        let (Guess(x), s) = s.receive().await?;
        let s = s.select(Correct(x)).await?;
        return Ok(((), s))
    })
    .await
}

async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: PlusMinusC<'_, _>| async {
        let s = s.send(Guess(10)).await?;
        match s.branch().await? {
            PlusMinusC2::Correct(Correct(_), s) => return Ok(((), s)),
            _ => panic!(),
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(a(&mut roles.a), b(&mut roles.b), c(&mut roles.c)).unwrap();
    });
}
