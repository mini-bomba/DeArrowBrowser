/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2025 mini_bomba
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

use std::sync::{Arc, LazyLock};

use actix_web::http::StatusCode;

/// This error extension sets the response status for error responses
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ResponseCodeExt(pub StatusCode);
impl cloneable_errors::Extension for ResponseCodeExt {}

// common response codes
macro_rules! lazy_ext {
    ($($status:ident),+) => {
    $(
        pub static $status: LazyLock<Arc<ResponseCodeExt>> = LazyLock::new(|| Arc::new(ResponseCodeExt(StatusCode::$status)));
    )+
    };
}

lazy_ext!(BAD_REQUEST, FORBIDDEN, NOT_FOUND);
