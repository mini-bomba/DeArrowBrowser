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

mod context;
mod iterator;
mod serializable;

pub use context::*;
pub use iterator::*;
pub use serializable::*;

#[macro_export]
/// Create a new [`ErrorContext`] stack
macro_rules! anyhow {
    ($val:expr) => {
        $crate::ErrorContext::new($val)
    };
    ($($tok:tt)+) => {
        $crate::ErrorContext::new(format!($($tok)+))
    };
}

#[macro_export]
/// Create a new [`ErrorContext`] stack and immediately return it as [`Result::Err`]
macro_rules! bail {
    ($($tok:tt)+) => {
        return Err($crate::anyhow!($($tok)+));
    };
}
