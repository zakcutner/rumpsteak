use crate::{cache::Cache, client::Client, proxy::Proxy, Channel, Request, Response, Result};
use hyper::{
    client::{self, HttpConnector},
    http::uri::{Authority, PathAndQuery, Scheme},
    Uri,
};
use rumpsteak::{channel::Nil, session, try_session, End, Receive, Role, Send};
use std::{any::Any, marker, mem};
use tracing::error;

#[derive(Role)]
#[message(Box<dyn Any + marker::Send>)]
pub struct Origin {
    #[route(Client)]
    pub(crate) client: Nil,
    #[route(Proxy)]
    pub(crate) proxy: Channel,
    #[route(Cache)]
    pub(crate) cache: Nil,
}

#[session]
type Session = Receive<Proxy, Request, Send<Proxy, Response, End>>;

fn set_authority(uri: &mut Uri, authority: Authority) -> Result<()> {
    let old = mem::take(uri).into_parts();
    *uri = Uri::builder()
        .scheme(old.scheme.unwrap_or(Scheme::HTTP))
        .authority(authority)
        .path_and_query(
            old.path_and_query
                .unwrap_or_else(|| PathAndQuery::from_static("")),
        )
        .build()?;
    Ok(())
}

async fn try_run(
    role: &mut Origin,
    remote: Authority,
    client: &client::Client<HttpConnector>,
) -> Result<()> {
    try_session(role, |s: Session<'_, Origin>| async {
        let (mut request, s) = s.receive().await?;
        set_authority(request.uri_mut(), remote)?;
        let response = client.request(request).await?;
        let s = s.send(Response::new(response).await?).await?;
        Ok(((), s))
    })
    .await
}

pub async fn run(
    role: &mut Origin,
    remote: Authority,
    client: &client::Client<HttpConnector>,
) -> Result<()> {
    let result = try_run(role, remote, client).await;
    if let Err(err) = &result {
        error!("{}", err);
    }

    result
}
