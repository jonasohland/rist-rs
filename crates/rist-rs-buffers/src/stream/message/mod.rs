use core::fmt::Debug;
use core::hash::Hash;

mod non_blocking;

pub trait MessageStreamPeerAddress: Clone + Copy + Hash + Debug + Eq {}

impl<T> MessageStreamPeerAddress for T where T: Clone + Copy + Hash + Debug + Eq {}

pub use non_blocking::NonBlockingMessageStream;
pub use non_blocking::NonBlockingMessageStreamAcceptor;
pub use non_blocking::NonBlockingMessageStreamConnector;
pub use non_blocking::NonBlockingMessageStreamError;
