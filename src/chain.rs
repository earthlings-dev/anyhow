//! Error chain iterator implementation.
//!
//! Provides iteration over the causal chain of an error, from the outermost
//! error down to the root cause, using `std::error::Error::source`.

use self::ChainState::*;
use crate::StdError;

use alloc::vec::{self, Vec};

pub(crate) use crate::Chain;

/// Internal state for the [`Chain`] iterator.
///
/// The iterator starts in `Linked` state, lazily following the error's source
/// chain. When reverse iteration is requested, it transitions to `Buffered`
/// state by collecting all remaining errors into a vector.
#[derive(Clone)]
pub(crate) enum ChainState<'a> {
    /// Lazily traverses the error chain via `source()` calls.
    Linked {
        next: Option<&'a (dyn StdError + 'static)>,
    },
    /// Buffered errors for reverse iteration support.
    Buffered {
        rest: vec::IntoIter<&'a (dyn StdError + 'static)>,
    },
}

impl<'a> Chain<'a> {
    /// Creates a new chain iterator starting from the given error.
    #[cold]
    pub fn new(head: &'a (dyn StdError + 'static)) -> Self {
        Chain {
            state: ChainState::Linked { next: Some(head) },
        }
    }
}

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            Linked { next } => {
                let error = (*next)?;
                *next = error.source();
                Some(error)
            }
            Buffered { rest } => rest.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Chain<'_> {
    /// Returns the next error from the end of the chain (root cause first).
    ///
    /// On first call, this collects all remaining errors into a buffer to
    /// enable bidirectional iteration.
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            &mut Linked { mut next } => {
                let mut rest = Vec::new();
                while let Some(cause) = next {
                    next = cause.source();
                    rest.push(cause);
                }
                let mut rest = rest.into_iter();
                let last = rest.next_back();
                self.state = Buffered { rest };
                last
            }
            Buffered { rest } => rest.next_back(),
        }
    }
}

impl ExactSizeIterator for Chain<'_> {
    /// Computes the number of errors in the chain.
    ///
    /// Note: In `Linked` state, this traverses the entire chain to count.
    fn len(&self) -> usize {
        match &self.state {
            &Linked { mut next } => {
                let mut len = 0;
                while let Some(cause) = next {
                    next = cause.source();
                    len += 1;
                }
                len
            }
            Buffered { rest } => rest.len(),
        }
    }
}

impl Default for Chain<'_> {
    /// Creates an empty chain iterator.
    fn default() -> Self {
        Chain {
            state: ChainState::Buffered {
                rest: Vec::new().into_iter(),
            },
        }
    }
}
