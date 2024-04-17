async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: PlusMinusA<'_, _>| async {
        let s = s.send(Secret(10)).await?;
        return Ok(((), s))
    })
    .await
}

async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: PlusMinusB<'_, _>| async {
        let (Secret(n), mut s) = s.receive().await?;

	loop {
		let (Guess(x), s1) = s.receive().await?;
		if n > x {
			s = s1.select(More(x)).await?;
		} else if n < x {
			s = s1.select(Less(x)).await?;
		} else {
			let s = s1.select(Correct(x)).await?;
			return Ok(((), s))
		}
	}
    })
    .await
}

async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: PlusMinusC<'_, _>| async {
	let mut min = i32::MIN;
	let mut max = i32::MAX; // both included
	let mut s = s;
	loop {
		let attempt = min/2 + max/2;
		let s1 = s.send(Guess(attempt)).await?;
		match s1.branch().await? {
		PlusMinusC2::Correct(Correct(_), s_end) => {
			println!("Final guess {}", attempt);
			return Ok(((), s_end))
		},
		PlusMinusC2::Less(_, s_cont) => { s = s_cont; max = attempt - 1; },
		PlusMinusC2::More(_, s_cont) => { s = s_cont; min = attempt + 1; },
		}
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
