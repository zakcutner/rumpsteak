use crate::{client::Client, origin::Origin, proxy::Proxy, wrap, Channel, Entry, Result};
use fred::{
    client::RedisClient,
    types::{RedisValue, SetOptions::NX},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rumpsteak::{channel::Nil, session, try_session, Branch, End, Receive, Role, Send};
use std::{any::Any, marker, time::Duration};
use tokio::time;
use tracing::{debug, error};

const MAX_RETRY_DELAY: u64 = 1000;

#[derive(Role)]
#[message(Box<dyn Any + marker::Send>)]
pub struct Cache {
    #[route(Client)]
    pub(crate) client: Nil,
    #[route(Proxy)]
    pub(crate) proxy: Channel,
    #[route(Origin)]
    pub(crate) origin: Nil,
}

pub struct Lock(pub(crate) String);

pub struct Locked;

pub struct Unlock;

pub struct Load;

pub struct Store(pub(crate) Entry);

pub struct Remove;

#[session]
type Session = Receive<Proxy, Lock, Send<Proxy, Locked, Branch<Proxy, Choice>>>;

#[session]
enum Choice {
    Load(Load, Send<Proxy, Option<Entry>, Branch<Proxy, Choice>>),
    Store(Store, Branch<Proxy, Choice>),
    Remove(Remove, Branch<Proxy, Choice>),
    Unlock(Unlock, End),
}

enum Replica {
    None,
    Clean(Option<Entry>),
    Dirty(Option<Entry>),
}

async fn try_run(role: &mut Cache, redis: &RedisClient) -> Result<()> {
    let mut rng = StdRng::from_entropy();
    try_session(role, |s: Session<'_, Cache>| async {
        let (Lock(mut lock), s) = s.receive().await?;
        let size = lock.len();
        lock.push_str(":lock");
        let key = &lock[..size];

        while !wrap(redis.set(&lock, 0, None, Some(NX), false).await)?.is_ok() {
            let delay = rng.gen_range(0..MAX_RETRY_DELAY);
            time::sleep(Duration::from_millis(delay)).await;
        }

        let mut s = s.send(Locked).await?;
        let mut replica = Replica::None;

        loop {
            s = match s.branch().await? {
                Choice::Load(Load, s) => loop {
                    match &replica {
                        Replica::None => {
                            let entry = wrap(redis.get(key).await)?;
                            replica = Replica::Clean(match entry.as_bytes() {
                                Some(bytes) => Some(bincode::deserialize(bytes)?),
                                None => None,
                            });
                        }
                        Replica::Clean(entry) | Replica::Dirty(entry) => {
                            break s.send(entry.clone()).await?
                        }
                    }
                },
                Choice::Store(Store(cacheable), s) => {
                    debug!("storing a new cache entry");
                    replica = Replica::Dirty(Some(cacheable));
                    s
                }
                Choice::Remove(Remove, s) => {
                    debug!("removing a cache entry");
                    replica = Replica::Dirty(None);
                    s
                }
                Choice::Unlock(Unlock, s) => {
                    if let Replica::Dirty(entry) = replica {
                        debug!("flushing changes to cache");
                        match entry {
                            Some(entry) => {
                                let value = RedisValue::Bytes(bincode::serialize(&entry)?);
                                let value = wrap(redis.set(key, value, None, None, false).await)?;
                                assert!(value.is_ok());
                            }
                            None => {
                                assert_eq!(wrap(redis.del(key).await)?, RedisValue::Integer(1));
                            }
                        };
                    }

                    assert_eq!(wrap(redis.del(&lock).await)?, RedisValue::Integer(1));
                    return Ok(((), s));
                }
            }
        }
    })
    .await
}

pub async fn run(role: &mut Cache, redis: &RedisClient) -> Result<()> {
    let result = try_run(role, redis).await;
    if let Err(err) = &result {
        error!("{}", err);
    }

    result
}
