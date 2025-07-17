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

use std::{collections::HashSet, sync::Arc};


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
