use crate::{cache::Cache, origin::Origin, proxy::Proxy, Channel, Request, Response, Result};
use hyper::Body;
use rumpsteak::{channel::Nil, session, try_session, End, Receive, Role, Send};
use std::{any::Any, marker};
use tracing::error;

#[derive(Role)]
#[message(Box<dyn Any + marker::Send>)]
pub struct Client {
    #[route(Proxy)]
    pub(crate) proxy: Channel,
    #[route(Cache)]
    pub(crate) cache: Nil,
    #[route(Origin)]
    pub(crate) origin: Nil,
}

#[session]
type Session = Send<Proxy, Request, Receive<Proxy, Response, End>>;

async fn try_run(
    role: &mut Client,
    request: hyper::Request<Body>,
) -> Result<hyper::Response<Body>> {
    try_session(role, |s: Session<'_, Client>| async {
        let s = s.send(request).await?;
        let (response, s) = s.receive().await?;
        Ok((response.into_response(), s))
    })
    .await
}

pub async fn run(
    role: &mut Client,
    request: hyper::Request<Body>,
) -> Result<hyper::Response<Body>> {
    let result = try_run(role, request).await;
    if let Err(err) = &result {
        error!("{}", err);
    }

    result
}
