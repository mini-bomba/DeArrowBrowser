/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
*  
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::{convert::Infallible, error::Error, fmt::{Debug, Display}, sync::Arc};

use crate::IntoErrorIterator;

// The ErrorContext struct

#[derive(Clone)]
pub struct ErrorContext {
    pub context: Arc<str>,
    pub cause: Option<Arc<dyn Error + Send + Sync + 'static>>,
}

impl ErrorContext {
    pub fn new<T>(msg: T) -> ErrorContext
    where T: Into<Arc<str>>
    {
        ErrorContext { context: msg.into(), cause: None }
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl Debug for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.error_chain();
        write!(f, "{}", iter.next().expect("first item should exist"))?;
        
        let mut iter = iter.enumerate();
        if let Some((i, item)) = iter.next() {
            write!(f, "\n\nCaused by:\n    {i}: {item}")?;

            for (i, item) in iter {
                write!(f, "\n    {i}: {item}")?;
            }
        }

        Ok(())
        
    }
}

impl Error for ErrorContext {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Some(cause) = self.cause.as_deref() {
            Some(cause)
        } else {
            None
        }
    }
}



// .context() traits
// on all Errors

pub trait ErrContext {
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<Arc<str>>;
}

impl<T> ErrContext for T
where T: Error + Send + Sync + 'static
{
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<Arc<str>>
    {
        ErrorContext {
            context: msg.into(),
            cause: Some(Arc::new(self)),
        }
    }
}


// on all Result<>s

pub trait ResContext<T, E> {
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>;

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>,
          F: FnOnce() -> M;
}

impl<T, E> ResContext<T, E> for Result<T, E>
where E: ErrContext
{
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>
    {
        self.map_err(|e| e.context(msg))
    }

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>,
          F: FnOnce() -> M 
    {
        self.map_err(|e| e.context(f()))
    }
}


// on all Option<>s

impl<T> ResContext<T, Infallible> for Option<T>
{
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>
    {
        self.ok_or_else(|| ErrorContext { context: msg.into(), cause: None })
    }

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<Arc<str>>,
          F: FnOnce() -> M 
    {
        self.ok_or_else(|| ErrorContext { context: f().into(), cause: None })
    }
}
