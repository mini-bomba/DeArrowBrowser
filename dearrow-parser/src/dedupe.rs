/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2025 mini_bomba
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

use std::{collections::HashSet, hash::{Hash, Hasher}, ops::Deref, sync::Arc};


#[derive(Default, Clone)]
pub struct StringSet {
    pub set: HashSet<Arc<str>>,
}

impl StringSet {
    pub fn with_capacity(capacity: usize) -> StringSet {
        StringSet {
            set: HashSet::with_capacity(capacity),
        }
    }

    pub fn dedupe_struct<T: Dedupe>(&mut self, obj: &mut T) {
        obj.dedupe(self);
    }

    pub fn dedupe_arc(&mut self, reference: &mut Arc<str>) {
        if let Some(s) = self.set.get(reference) {
            *reference = s.clone();
        } else {
            self.set.insert(reference.clone());
        }
    }

    pub fn clean(&mut self) {
        self.set.retain(|s| Arc::strong_count(s) > 1);
    }
}

pub trait Dedupe {
    fn dedupe(&mut self, set: &mut StringSet);
}

pub fn arc_addr<T: ?Sized>(arc: &Arc<T>) -> usize {
    Arc::as_ptr(arc).addr()
}

/// A wrapper around [`Arc<T>`] that redefines the equality comparisons to be simple pointer equality
/// checks.
#[derive(Clone, Debug)]
pub struct AddrArc<T: ?Sized>(Arc<T>);

impl<T: ?Sized> AddrArc<T> {
    /// Returns the internal Arc<T> instance
    #[must_use]
    pub fn unwrap(self) -> Arc<T> {
        self.0
    }
}

impl<T: ?Sized> PartialEq for AddrArc<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ?Sized> Eq for AddrArc<T> {}
impl<T: ?Sized> Hash for AddrArc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).addr().hash(state);
    }
}

impl<T: ?Sized> Deref for AddrArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> From<Arc<T>> for AddrArc<T> {
    fn from(value: Arc<T>) -> Self {
        Self(value)
    }
}
