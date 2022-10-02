#![allow(unused)]
use crate::transport::stream::SslContextProvider;

#[derive(Clone, Debug)]
pub struct Config<C>
where
    C: SslContextProvider,
{
    stream: crate::transport::stream::Config<C>,
}
