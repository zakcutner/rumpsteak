
async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: PingPongA<'_, _>| async {
        let mut x = 0;
        let mut s = s;
        loop {
            let cont_rec = s.0.send(Ping(x)).await?;
            let (Pong(y), cont) = cont_rec.receive().await?;
            s = cont;
            x = y;
            if y % 10000 == 0 {
                println!("Role A received {}", y);
            }
        }
    })
    .await
}

async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: PingPongB<'_, _>| async {
        let mut s = s;
        loop {
            let (Ping(x), cont_snd) = s.0.receive().await?;
            s = cont_snd.send(Pong(x+1)).await?;
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(a(&mut roles.a), b(&mut roles.b)).unwrap();
    });
}
