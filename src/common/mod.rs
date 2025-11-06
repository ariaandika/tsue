use std::task::Poll;

#[derive(Debug)]
pub enum ParseResult<T, E> {
    /// Bytes is not sufficient for parsing, more IO read is required.
    Pending,
    /// Parse success.
    Ok(T),
    /// Parse failed.
    Err(E),
}

impl<T, E> ParseResult<T, E> {
    /// Returns `true` if the parse result is [`Pending`].
    ///
    /// [`Pending`]: ParseResult::Pending
    #[inline]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the parse result is [`Ok`].
    ///
    /// [`Ok`]: ParseResult::Ok
    #[inline]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(..))
    }

    /// Returns `true` if the parse result is [`Err`].
    ///
    /// [`Err`]: ParseResult::Err
    #[inline]
    pub const fn is_err(&self) -> bool {
        matches!(self, Self::Err(..))
    }

    /// Convert to [`Poll<Result<T, E>>`].
    #[inline]
    pub fn into_poll_result(self) -> Poll<Result<T, E>> {
        match self {
            ParseResult::Pending => Poll::Pending,
            ParseResult::Ok(ok) => Poll::Ready(Ok(ok)),
            ParseResult::Err(err) => Poll::Ready(Err(err)),
        }
    }
}
