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

/// A helper enum for easily cloneable strings
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SharedString {
    Arc(Arc<str>),
    Static(&'static str),
}

impl Display for SharedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedString::Arc(s) => write!(f, "{s}"),
            SharedString::Static(s) => write!(f, "{s}"),
        }
    }
}

impl From<&'static str> for SharedString {
    fn from(value: &'static str) -> Self {
        SharedString::Static(value)
    }
}

impl From<Arc<str>> for SharedString {
    fn from(value: Arc<str>) -> Self {
        SharedString::Arc(value)
    }
}

impl From<String> for SharedString
{
    fn from(value: String) -> Self {
        SharedString::Arc(Arc::from(value))
    }
}

// The ErrorContext struct

#[derive(Clone)]
/// An annotated error stack
pub struct ErrorContext {
    pub context: SharedString,
    pub cause: Option<Arc<dyn Error + Send + Sync + 'static>>,
}

impl ErrorContext {
    pub fn new<T>(msg: T) -> ErrorContext
    where T: Into<SharedString>
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

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for ErrorContext {
    fn from(value: anyhow::Error) -> Self {
        let flattened = crate::SerializableError::from_anyhow(&value);
        ErrorContext { 
            context: SharedString::Arc(flattened.context),
            cause: flattened.cause.map(|arc| arc as Arc<(dyn Error + Send + Sync + 'static)>),
        }
    }
}


// .context() traits
// on all Errors

/// A helper trait for annotating any Error with an [`ErrorContext`]
pub trait ErrContext {
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<SharedString>;
}

impl<T> ErrContext for T
where T: Error + Send + Sync + 'static
{
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<SharedString>
    {
        ErrorContext {
            context: msg.into(),
            cause: Some(Arc::new(self)),
        }
    }
}

// on anyhow::Error

#[cfg(feature = "anyhow")]
/// A helper trait for converting an anyhow error stack into an [`ErrorContext`] stack
pub trait AnyhowErrContext {
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<SharedString>;
}

#[cfg(feature = "anyhow")]
impl AnyhowErrContext for anyhow::Error {
    fn context<M>(self, msg: M) -> ErrorContext
    where M: Into<SharedString> 
    {
        ErrorContext { 
            context: msg.into(),
            cause: Some(Arc::new(ErrorContext::from(self)))
        }    
    }
}

// on all Result<>s

/// A helper trait for annotating result errors and empty options
pub trait ResContext<T, E> {
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<SharedString>;

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<SharedString>,
          F: FnOnce() -> M;
}

impl<T, E> ResContext<T, E> for Result<T, E>
where E: ErrContext
{
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<SharedString>
    {
        self.map_err(|e| e.context(msg))
    }

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<SharedString>,
          F: FnOnce() -> M 
    {
        self.map_err(|e| e.context(f()))
    }
}

// on Result<>s with anyhow::Error
#[cfg(feature = "anyhow")]
/// A helper trait for converting anyhow results into [`ErrorContext`] results
pub trait AnyhowResContext<T, E> {
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<SharedString>;

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<SharedString>,
          F: FnOnce() -> M;
}

#[cfg(feature = "anyhow")]
impl<T, E> AnyhowResContext<T, E> for Result<T, E>
where E: AnyhowErrContext
{
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<SharedString>
    {
        self.map_err(|e| e.context(msg))
    }

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<SharedString>,
          F: FnOnce() -> M 
    {
        self.map_err(|e| e.context(f()))
    }
}

// on all Option<>s

impl<T> ResContext<T, Infallible> for Option<T>
{
    fn context<M>(self, msg: M) -> Result<T, ErrorContext>
    where M: Into<SharedString>
    {
        self.ok_or_else(|| ErrorContext { context: msg.into(), cause: None })
    }

    fn with_context<M, F>(self, f: F) -> Result<T, ErrorContext>
    where M: Into<SharedString>,
          F: FnOnce() -> M 
    {
        self.ok_or_else(|| ErrorContext { context: f().into(), cause: None })
    }
}
