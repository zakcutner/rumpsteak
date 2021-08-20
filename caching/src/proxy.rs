use crate::{
    cache::{Cache, Load, Lock, Locked, Remove, Store, Unlock},
    client::Client,
    origin::Origin,
    Channel, Entry, Request, Response, Result,
};
use hyper::{body::HttpBody, header::HeaderValue, Method, StatusCode, Uri};
use rumpsteak::{session, try_session, End, Receive, Role, Select, Send};
use serde::Serialize;
use std::{any::Any, marker};
use tracing::{debug, error};

#[derive(Role)]
#[message(Box<dyn Any + marker::Send>)]
pub struct Proxy {
    #[route(Client)]
    pub(crate) client: Channel,
    #[route(Cache)]
    pub(crate) cache: Channel,
    #[route(Origin)]
    pub(crate) origin: Channel,
}

#[session]
#[rustfmt::skip]
type Session = Receive<Client, Request, Send<Cache, Lock, Send<Cache, Load, Receive<Cache, Locked, Receive<Cache, Option<Entry>, Forward>>>>>;

#[session]
#[rustfmt::skip]
type Forward = Send<Origin, Request, Receive<Origin, Response, Send<Client, Response, Select<Cache, Choice>>>>;

#[session]
enum Choice {
    Store(Store, Select<Cache, Self>),
    Remove(Remove, Select<Cache, Self>),
    Unlock(Unlock, End),
}

#[derive(Serialize)]
struct Key<'a> {
    #[serde(with = "http_serde::method")]
    method: &'a Method,
    #[serde(with = "http_serde::uri")]
    uri: &'a Uri,
    headers: Vec<Option<&'a [u8]>>,
}

async fn respond(
    request: Request,
    cached: Option<Response>,
    remove: bool,
    s: Forward<'_, Proxy>,
) -> Result<Select<'_, Proxy, Cache, Choice<'_, Proxy>>> {
    let mut is_safe = false;
    if let Some(size) = request.body().size_hint().upper() {
        is_safe = request.method().is_safe() && size == 0;
    }

    let s = s.send(request).await?;
    let (response, s) = s.receive().await?;
    match cached {
        Some(cached) if response.status == StatusCode::NOT_MODIFIED => Ok(s.send(cached).await?),
        _ => match response.headers.get("ETag") {
            Some(etag) if is_safe => {
                debug!("found a cacheable response");
                let s = s.send(response.clone()).await?;
                let etag = etag.as_bytes().to_owned();
                Ok(s.select(Store(Entry { etag, response })).await?)
            }
            _ => {
                let mut s = s.send(response).await?;
                if remove {
                    s = s.select(Remove).await?;
                }

                Ok(s)
            }
        },
    }
}

async fn try_run(role: &mut Proxy, headers: &[String]) -> Result<()> {
    try_session(role, |s: Session<'_, Proxy>| async {
        let (mut request, s) = s.receive().await?;
        let mut key = format!("{}:{}", request.method(), request.uri());
        for header in headers {
            key.push(':');
            if let Some(value) = request.headers().get(header) {
                key.push_str(&String::from_utf8_lossy(value.as_bytes()));
            }
        }

        let s = s.send(Lock(key)).await?.send(Load).await?;
        let (Locked, s) = s.receive().await?;

        let s = match s.receive().await? {
            (Some(entry), s) => {
                let etag = HeaderValue::from_bytes(&entry.etag)?;
                request.headers_mut().append("If-None-Match", etag);
                respond(request, Some(entry.response), true, s).await?
            }
            (None, s) => respond(request, None, false, s).await?,
        };

        Ok(((), s.select(Unlock).await?))
    })
    .await
}

pub async fn run(role: &mut Proxy, headers: &[String]) -> Result<()> {
    let result = try_run(role, headers).await;
    if let Err(err) = &result {
        error!("{}", err);
    }

    result
}
