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

use std::{error::Error, fmt::{Display, Debug}, sync::Arc};

#[cfg(feature="serde")]
use serde::{Deserialize, Serialize};

use crate::IntoErrorIterator;


// SerializableError - an error stack with all messages flattened into strings, trivial to
// (de)serialize
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SerializableError {
    pub context: Arc<str>,
    pub cause: Option<Arc<SerializableError>>,
}

impl Display for SerializableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context)
    }
}

impl Debug for SerializableError {
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

impl Error for SerializableError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Some(cause) = self.cause.as_deref() {
            Some(cause)
        } else {
            None
        }
    }
}

#[cfg(feature = "anyhow")]
impl SerializableError {
    pub fn from_anyhow(err: &anyhow::Error) -> Self {
        let mut iter = err.chain();
        let mut result = SerializableError {
            context: format!("{}", iter.next().expect("first item should exist")).into(),
            cause: None,
        };
        let mut last = &mut result;

        for err in iter {
            last.cause = Some(Arc::new(SerializableError { context: format!("{err}").into(), cause: None }));
            last = Arc::get_mut(last.cause.as_mut().unwrap()).unwrap();
        }

        result
    }
}
