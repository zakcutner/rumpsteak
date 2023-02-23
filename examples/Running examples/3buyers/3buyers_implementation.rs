async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersB<'_, _>| async {
        let (QuoteBob(msg_s), s) = s.receive().await?;
        let (ParticipationBob(msg_a), s) = s.receive().await?;
        if msg_a == msg_s {
            // Accept command if both prices are the same
            let s = s.select(ConfirmAlice(msg_s)).await?;
            let s = s.send(ConfirmSeller(msg_s)).await?;
            let (Date(_msg), s) = s.receive().await?;
            println!("Accept order (price {})", msg_a);
            Ok(((), s))
        } else {
            let s = s.select(QuitAlice(0)).await?;
            let s = s.send(QuitSeller(0)).await?;
            println!("Reject order (price inconsistency {} vs {})", msg_a, msg_s);
            Ok(((), s))
        }
    })
    .await
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersS<'_, _>| async {
        let (Request(msg), s) = s.receive().await?;
        let s = s.send(QuoteAlice(msg)).await?;
        let s = s.send(QuoteBob(msg)).await?;
        match s.branch().await? {
            ThreeBuyersS3::ConfirmSeller(msg, s) => {
                let s = s.send(Date(42)).await?;
                Ok(((), s))
            }
            ThreeBuyersS3::QuitSeller(_, end) => Ok(((), end)),
        }
    })
    .await
}

async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersA<'_, _>| async {
        let s = s.send(Request(42)).await?;
        let (_reply, s) = s.receive().await?;
        let s = s.send(ParticipationBob(42)).await?;
        match s.branch().await? {
            ThreeBuyersA3::ConfirmAlice(_, end) => Ok(((), end)),
            ThreeBuyersA3::QuitAlice(_, end) => Ok(((), end)),
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(b(&mut roles.b), s(&mut roles.s), a(&mut roles.a)).unwrap();
    });
}
