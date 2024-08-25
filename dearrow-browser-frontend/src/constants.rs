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

use reqwest::{Url, Client};

pub static REQWEST_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

// URLs

pub static YOUTU_BE_URL:       LazyLock<Url> = LazyLock::new(|| Url::parse("https://youtu.be/").expect("should be able to parse youtu.be base URL"));
pub static YOUTUBE_OEMBED_URL: LazyLock<Url> = LazyLock::new(|| Url::parse("https://www.youtube-nocookie.com/oembed").expect("should be able to parse youtube-nocookie oembed URL"));
pub static YOUTUBE_EMBED_URL:  LazyLock<Url> = LazyLock::new(|| Url::parse("https://www.youtube-nocookie.com/embed/").expect("should be able to parse the youtube embed url"));
pub static THUMBNAIL_URL:      LazyLock<Url> = LazyLock::new(|| Url::parse("https://img.youtube.com/vi").expect("should be able to parse the youtube thumbnail URL"));
pub static SBB_BASE:           LazyLock<Url> = LazyLock::new(|| Url::parse("https://sb.ltn.fi/").expect("should be able to parse sb.ltn.fi base URL"));
