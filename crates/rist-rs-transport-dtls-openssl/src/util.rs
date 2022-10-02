use rist_rs_buffers::stream::message::NonBlockingMessageStreamError;
use std::io;

/// Converts the result of a NonBlockingMessageStream result to an std::io result
pub fn non_blocking_stream_to_io_result<R>(
    r: Option<Result<R, NonBlockingMessageStreamError>>,
) -> io::Result<R> {
    match r {
        None => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        Some(Err(NonBlockingMessageStreamError::Closed)) => {
            Err(io::Error::from(io::ErrorKind::ConnectionAborted))
        }
        Some(Ok(result)) => Ok(result),
    }
}
