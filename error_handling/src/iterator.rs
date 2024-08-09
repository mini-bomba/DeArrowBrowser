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

use std::{error::Error, sync::Arc};

use crate::SerializableError;


/// `ErrorIterator` - iterates over the chain of `Error::source`
pub struct ErrorIterator<'a> {
    next_item: Option<&'a (dyn Error + 'static)>,
}

impl<'a> Iterator for ErrorIterator<'a> {
    type Item = &'a (dyn Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(err) = self.next_item {
            self.next_item = err.source();
            Some(err)
        } else {
            None
        }
    }
}

pub trait IntoErrorIterator {
    fn error_chain(&self) -> ErrorIterator<'_>;

    fn serializable_copy(&self) -> SerializableError {
        let mut iter = self.error_chain();
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

impl<T> IntoErrorIterator for T
where T: Error + 'static
{
    fn error_chain(&self) -> ErrorIterator<'_> {
        ErrorIterator { next_item: Some(self) }
    }
}
