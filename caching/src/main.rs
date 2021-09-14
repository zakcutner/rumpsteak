mod cache;
mod client;
mod origin;
mod proxy;

use crate::{cache::Cache, client::Client, origin::Origin, proxy::Proxy};
use argh::FromArgs;
use fred::{
    client::RedisClient,
    types::{ReconnectPolicy, RedisConfig, ServerConfig},
};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    Future,
};
use hyper::{
    body::{self, Bytes},
    client::HttpConnector,
    http::uri::Authority,
    service::{make_service_fn, service_fn},
    Body, HeaderMap, Server, StatusCode,
};
use rumpsteak::{channel::Bidirectional, Roles};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    convert::Infallible,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    result,
    sync::Arc,
};
use tokio::try_join;
use tracing::{debug, info};

const DEFAULT_HEADERS: &[&str] = &["Cookie", "Host"];

type Result<T, E = Box<dyn Error + Send + Sync>> = result::Result<T, E>;

type Channel =
    Bidirectional<UnboundedSender<Box<dyn Any + Send>>, UnboundedReceiver<Box<dyn Any + Send>>>;

type Request = hyper::Request<Body>;

struct ErrorWrapper<E>(E);

impl<E: Debug> Debug for ErrorWrapper<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: Display> Display for ErrorWrapper<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: Debug + Display> Error for ErrorWrapper<E> {}

fn wrap<T, E: Debug + Display>(result: Result<T, E>) -> Result<T, ErrorWrapper<E>> {
    result.map_err(ErrorWrapper)
}

#[derive(Roles)]
struct Roles {
    client: Client,
    proxy: Proxy,
    cache: Cache,
    origin: Origin,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Response {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
    body: Bytes,
}

impl Response {
    async fn new(response: hyper::Response<Body>) -> hyper::Result<Self> {
        let (head, body) = response.into_parts();
        Ok(Self {
            status: head.status,
            headers: head.headers,
            body: body::to_bytes(body).await?,
        })
    }

    fn into_response(self) -> hyper::Response<Body> {
        let mut response = hyper::Response::new(self.body.into());
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entry {
    etag: Vec<u8>,
    response: Response,
}

#[derive(FromArgs)]
/// An HTTP cache backed by session types.
struct Options {
    /// set the IP address to listen on (defaults to 127.0.0.1)
    #[argh(option, short = 'l', default = "IpAddr::V4(Ipv4Addr::LOCALHOST)")]
    listen: IpAddr,

    /// set the port to listen on (defaults to port 3000)
    #[argh(option, short = 'p', default = "3000")]
    port: u16,

    /// add a header's value to the cache key (the values of the 'Cache' and
    /// 'Host' headers are always included)
    #[argh(option, short = 'H')]
    header: Vec<String>,

    /// set the authority to connect to Redis on (defaults to 127.0.0.1:6379)
    #[argh(option, short = 'r', default = "Authority::from_static(\"127.0.0.1\")")]
    redis: Authority,

    /// set the remote authority for forwarding (e.g. example.com)
    #[argh(positional)]
    remote: Authority,
}

#[derive(Clone)]
struct Context {
    headers: Vec<String>,
    redis: RedisClient,
    remote: Authority,
    hyper: hyper::Client<HttpConnector>,
}

async fn spawn<T: Send + 'static, F>(f: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
{
    tokio::spawn(f).await?
}

async fn handler(
    context: Arc<Context>,
    request: hyper::Request<Body>,
) -> Result<hyper::Response<Body>> {
    debug!("new request from client");
    let Roles {
        mut client,
        mut proxy,
        mut cache,
        mut origin,
    } = Roles::default();

    let client = spawn(async move { client::run(&mut client, request).await });
    let proxy = spawn({
        let context = context.clone();
        async move { proxy::run(&mut proxy, &context.clone().headers).await }
    });
    let cache = spawn({
        let context = context.clone();
        async move { cache::run(&mut cache, &context.redis).await }
    });
    let origin = spawn({
        let context = context.clone();
        async move { origin::run(&mut origin, context.remote.clone(), &context.hyper).await }
    });

    let (response, ..) = try_join!(client, proxy, cache, origin)?;
    Ok(response)
}

async fn try_main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut options = argh::from_env::<Options>();

    let default_headers = DEFAULT_HEADERS.iter().copied().map(ToOwned::to_owned);
    options.header.extend(default_headers);
    options.header.sort_unstable();
    options.header.dedup();

    let redis = RedisClient::new(RedisConfig {
        server: ServerConfig::new_centralized(
            options.redis.host(),
            options.redis.port_u16().unwrap_or(6379),
        ),
        ..Default::default()
    });
    let handle = redis.connect(Some(ReconnectPolicy::new_constant(0, 0)));
    wrap(redis.wait_for_connect().await)?;

    let context = Arc::new(Context {
        headers: options.header,
        redis: redis.clone(),
        remote: options.remote,
        hyper: hyper::Client::new(),
    });

    let make_service = make_service_fn(move |_| {
        let context = context.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |request| {
                let context = context.clone();
                async move {
                    Ok::<_, Infallible>(match handler(context, request).await {
                        Ok(response) => response,
                        Err(_) => hyper::Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap(),
                    })
                }
            }))
        }
    });

    let address = SocketAddr::new(options.listen, options.port);
    let server = Server::bind(&address).serve(make_service);

    info!("listening for connections on http://{}", address);
    server.await?;

    wrap(redis.quit().await)?;
    wrap(handle.await?)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("Error: {}", err);
    }
}
