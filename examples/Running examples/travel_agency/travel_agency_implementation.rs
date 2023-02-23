async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();

    try_session(role, map, |s: TravelAgencyC<'_, _>| async {
        let distance = 3; // destination
        let s = s.send(Order(distance)).await?;
        let (Quote(n), s) = s.receive().await?;
        if n < 100 {
            // Accept command if both prices are the same
            let s = s.select(Accept(0)).await?;
            let s = s.send(Address(1)).await?;
            let (Date(date), s) = s.receive().await?;
            println!("Client: Accept order (price {}, Date {})", n, date);
            Ok(((), s))
        } else {
            let s = s.select(Reject(-1)).await?;
            println!("Client: Reject order (too expensive {}, max {})", n, 100);
            Ok(((), s))
        }
    })
    .await
}

async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();

    try_session(role, map, |s: TravelAgencyA<'_, _>| async {
        let (Order(order), s) = s.receive().await?;
        let s = s.send(Quote(order*10)).await?; // Say the price is 10 per distance unit
        match s.branch().await? {
            TravelAgencyA2::Accept(_, s) => {
                let (Address(addr), s) = s.receive().await?;
                let s = s.send(Date(42)).await?; // Day of the year you leave
                println!("Agency: Reveive order (place {}, Customer address {})", order, addr);
                Ok(((), s))
            }
            TravelAgencyA2::Reject(_, end) => Ok(((), end)),
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(c(&mut roles.c), a(&mut roles.a)).unwrap();
    });
}
