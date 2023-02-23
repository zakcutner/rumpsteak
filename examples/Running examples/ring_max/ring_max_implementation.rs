
use std::cmp::{max, min};

async fn send_first_a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxA<'_, _>| async {
        let value = 10;

        let s = s.send(ProposalA(value)).await?;
        let (ProposalG(max), s) = s.receive().await?;

        println!("Max {}", max);

        return Ok(((), s))
    })
    .await
}

async fn receive_first_b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxB<'_, _>| async {
        let value = 15;

        let (ProposalA(a), s) = s.receive().await?;
        let s = s.send(ProposalB(max(value, a))).await?;

        return Ok(((), s))
    })
    .await
}

async fn receive_first_c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxC<'_, _>| async {
        let value = 15;

        let (ProposalB(b), s) = s.receive().await?;
        let s = s.send(ProposalC(max(value, b))).await?;

        return Ok(((), s))
    })
    .await
}

async fn receive_first_d(role: &mut D) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxD<'_, _>| async {
        let value = 15;

        let (ProposalC(c), s) = s.receive().await?;
        let s = s.send(ProposalD(max(value, c))).await?;

        return Ok(((), s))
    })
    .await
}

async fn receive_first_e(role: &mut E) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxE<'_, _>| async {
        let value = 15;

        let (ProposalD(d), s) = s.receive().await?;
        let s = s.send(ProposalE(max(value, d))).await?;

        return Ok(((), s))
    })
    .await
}

async fn receive_first_f(role: &mut F) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxF<'_, _>| async {
        let value = 15;

        let (ProposalE(e), s) = s.receive().await?;
        let s = s.send(ProposalF(max(value, e))).await?;

        return Ok(((), s))
    })
    .await
}

async fn receive_first_g(role: &mut G) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: RingMaxG<'_, _>| async {
        let value = 15;

        let (ProposalF(f), s) = s.receive().await?;
        let s = s.send(ProposalG(max(value, f))).await?;

        return Ok(((), s))
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(
            send_first_a(&mut roles.a),
            receive_first_b(&mut roles.b),
            receive_first_c(&mut roles.c),
            receive_first_d(&mut roles.d),
            receive_first_e(&mut roles.e),
            receive_first_f(&mut roles.f),
            receive_first_g(&mut roles.g)
        ).unwrap();
    });
}
