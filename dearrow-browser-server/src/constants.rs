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
use std::sync::LazyLock;
use error_handling::{ErrorContext, anyhow};

pub static SS_READ_ERR:  LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire StringSet for reading"));
pub static SS_WRITE_ERR: LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire StringSet for writing"));
pub static DB_READ_ERR:  LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire DatabaseState for reading"));
pub static DB_WRITE_ERR: LazyLock<ErrorContext> = LazyLock::new(|| anyhow!("Failed to acquire DatabaseState for writing"));
