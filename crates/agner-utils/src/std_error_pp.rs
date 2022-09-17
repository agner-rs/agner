use std::error::Error as StdError;
use std::fmt;

const DEFAULT_DEPTH_LIMIT: usize = 10;

pub trait StdErrorPP: StdError + Sized {
    fn pp<'a>(&'a self) -> ErrorPP<'a> {
        let err: &'a dyn StdError = self;
        ErrorPP { err, depth_limit: DEFAULT_DEPTH_LIMIT }
    }
}
impl<E> StdErrorPP for E where E: StdError + Sized {}

#[derive(Debug, Clone, Copy)]
pub struct ErrorPP<'a> {
    err: &'a dyn StdError,
    depth_limit: usize,
}

impl ErrorPP<'_> {
    pub fn depth_limit(self, depth_limit: usize) -> Self {
        Self { depth_limit, ..self }
    }
}

impl<'a> fmt::Display for ErrorPP<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut depth_left = self.depth_limit;
        let mut err = self.err;
        write!(f, "{}", err)?;

        while let Some(next) = err.source() {
            if depth_left == 0 {
                write!(f, " << ...")?;
            } else {
                depth_left -= 1;
                err = next;
                write!(f, " << {}", err)?;
            }
        }
        Ok(())
    }
}
