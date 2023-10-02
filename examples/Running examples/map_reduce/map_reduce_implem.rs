type Array = [u32; 5];

async fn c1(role: &mut C1) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: MapReduceC1<'_, _>| async {
        let mut s = s;
        loop {
            let s_branch = match s.branch().await? {
                MapReduceC10::Terminates(Terminates, s_end) => return Ok(((), s_end)),
                MapReduceC10::Workload(Workload(payload), s_return) => {
                    let sum = payload.iter().fold(0, |acc, n| acc + n);
                    let s_loop = s_return.send(PartialResult(sum)).await?;
                    s_loop
                }
            };
            s = s_branch;
        }
    })
    .await
}

async fn c2(role: &mut C2) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: MapReduceC2<'_, _>| async {
        let mut s = s;
        loop {
            let s_branch = match s.branch().await? {
                MapReduceC20::Terminates(Terminates, s_end) => return Ok(((), s_end)),
                MapReduceC20::Workload(Workload(payload), s_return) => {
                    let sum = payload.iter().fold(0, |acc, n| acc + n);
                    let s_loop = s_return.send(PartialResult(sum)).await?;
                    s_loop
                }
            };
            s = s_branch;
        }
    })
    .await
}

async fn c3(role: &mut C3) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: MapReduceC3<'_, _>| async {
        let mut s = s;
        loop {
            let s_branch = match s.branch().await? {
                MapReduceC30::Terminates(Terminates, s_end) => return Ok(((), s_end)),
                MapReduceC30::Workload(Workload(payload), s_return) => {
                    let sum = payload.iter().fold(0, |acc, n| acc + n);
                    let s_loop = s_return.send(PartialResult(sum)).await?;
                    s_loop
                }
            };
            s = s_branch;
        }
    })
    .await
}

async fn c4(role: &mut C4) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: MapReduceC4<'_, _>| async {
        let mut s = s;
        loop {
            let s_branch = match s.branch().await? {
                MapReduceC40::Terminates(Terminates, s_end) => return Ok(((), s_end)),
                MapReduceC40::Workload(Workload(payload), s_return) => {
                    let sum = payload.iter().fold(0, |acc, n| acc + n);
                    let s_loop = s_return.send(PartialResult(sum)).await?;
                    s_loop
                }
            };
            s = s_branch;
        }
    })
    .await
}

async fn server(role: &mut Server) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: MapReduceServer<'_, _>| async {
        let s = s.select(Workload([1, 2, 3, 4, 5])).await?;
        let s = s.send(Workload([6, 7, 8, 9, 10])).await?;
        let s = s.send(Workload([11, 12, 13, 14, 15])).await?;
        let s = s.send(Workload([16, 17, 18, 19, 20])).await?;

        let (PartialResult(sum_c1), s) = s.receive().await?;
        let (PartialResult(sum_c2), s) = s.receive().await?;
        let (PartialResult(sum_c3), s) = s.receive().await?;
        let (PartialResult(sum_c4), s) = s.receive().await?;

        let sum = sum_c1 + sum_c2 + sum_c3 + sum_c4;

        println!("The sum is: {sum}");

        let s = s.select(Terminates).await?;
        let s = s.send(Terminates).await?;
        let s = s.send(Terminates).await?;
        let s = s.send(Terminates).await?;
        return Ok(((), s));
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(
            server(&mut roles.server),
            c1(&mut roles.c1),
            c2(&mut roles.c2),
            c3(&mut roles.c3),
            c4(&mut roles.c4)
        )
        .unwrap();
    });
}
