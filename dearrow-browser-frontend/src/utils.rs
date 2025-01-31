/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
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
use std::{ops::Deref, rc::Rc, fmt::Write};

use chrono::{DateTime, Utc, NaiveDateTime};
use cloneable_errors::{bail, ErrContext, ErrorContext, ResContext, SerializableError};
use reqwest::Url;
use sha2::{digest::array::Array, Digest, Sha256};
use yew::Html;

use crate::constants::{REQWEST_CLIENT, SBB_BASE};

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn render_datetime(dt: DateTime<Utc>) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_naive_datetime(dt: NaiveDateTime) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_datetime_with_delta(dt: DateTime<Utc>) -> String
{
    format!("{} UTC ({} minutes ago)", dt.format(TIME_FORMAT), (Utc::now()-dt).num_minutes())
}

pub trait RenderNumber {
    /// Render a large integer in a human-readable way:
    /// Digits will be arranged in groups of 3, with spaces in between
    fn render_int(&self) -> String;
    /// Render a large integer in an abbreviated form:
    /// for example: 21370 will become 21K
    fn abbreviate_int(&self) -> String;
}

macro_rules! define_render_int {
    (unsigned) => {
        fn render_int(&self) -> String {
            let string_n = format!("{self}");
            let chunks = string_n.as_bytes() // get a bytes slice (cause chunking Iterators is nightly-only)
                .rchunks(3)            // make chunks of 3, starting from end. digits are ASCII = 1B each
                .rev()                 // reverse order of chunks
                .collect::<Vec<_>>();  // collect into a vec (cause intersperse is nightly-only)
            String::from_utf8(
                chunks.join(b" " as &[u8])  // separate chunks with a space
            ).expect("this should always be valid utf8")  // parse as string
        }
    };
    (signed) => {
        fn render_int(&self) -> String {
            let string_n = format!("{}", self.abs());
            let chunks = string_n.as_bytes() // get a bytes slice (cause chunking Iterators is nightly-only)
                .rchunks(3)            // make chunks of 3, starting from end
                .rev()                 // reverse order of chunks
                .collect::<Vec<_>>();  // collect into a vec (cause intersperse is nightly-only)
            let mut bytes = chunks.join(b" " as &[u8]);  // separate chunks with a space
            if self.is_negative() {
                bytes.insert(0, b'-');  // insert sign if needed
            }
            String::from_utf8(bytes)  // parse as string
                .expect("this should always be valid utf8")  // digits are ASCII, and always fit into 1 byte
        }
    };
}

macro_rules! define_big_abbreviate_int {
    (unsigned) => {
        #[allow(unreachable_patterns)]
        fn abbreviate_int(&self) -> String {
            match self {
                (0..=999) => format!("{self}"),
                (1_000..=999_999) => format!("{}K", self/1_000),
                (1_000_000..=999_999_999) => format!("{}M", self/1_000_000),
                (1_000_000_000..) => format!("{}B", (self/1_000_000_000).render_int()),
                _ => unreachable!(), // alredy covered by conditions above, required for compilation on aarch64
            }
        }
    };
    (signed) => {
        #[allow(unreachable_patterns)]
        fn abbreviate_int(&self) -> String {
            match self {
                (0..=999) => format!("{self}"),
                (1_000..=999_999) => format!("{}K", self/1_000),
                (1_000_000..=999_999_999) => format!("{}M", self/1_000_000),
                (1_000_000_000..) => format!("{}B", (self/1_000_000_000).render_int()),
                _ => unreachable!(), // alredy covered by conditions above, required for compilation on aarch64
            }
        }
    };
}

macro_rules! define_big_render_number {
    ($signedness: tt, $type: ident) => {
        impl RenderNumber for $type {
            define_render_int!($signedness);
            define_big_abbreviate_int!($signedness);
        }
    };
}

impl RenderNumber for u8 {
    fn render_int(&self) -> String {
        format!("{self}") // u8's are <=255, so will never reach 4 digits
    }
    fn abbreviate_int(&self) -> String {
        format!("{self}") // u8's are <=255, so will never reach 4 digits
    }
}
impl RenderNumber for i8 {
    fn render_int(&self) -> String {
        format!("{self}") // i8's are <=127, so will never reach 4 digits
    }
    fn abbreviate_int(&self) -> String {
        format!("{self}") // i8's are <=127, so will never reach 4 digits
    }
}
impl RenderNumber for u16 {
    define_render_int!(unsigned);
    fn abbreviate_int(&self) -> String {
        match self {
            (0..=999) => format!("{self}"),
            (1_000..=65_535) => format!("{}K", self/1_000),
        }
    }
}
impl RenderNumber for i16 {
    define_render_int!(signed);
    fn abbreviate_int(&self) -> String {
        match self {
            (0..=999) => format!("{self}"),
            (1_000..=32_767) => format!("{}K", self/1_000),
            (..=-1) => format!("-{}", self.abs()),
        }
    }
}
define_big_render_number!(unsigned, u32);
define_big_render_number!(unsigned, u64);
define_big_render_number!(unsigned, u128);
define_big_render_number!(unsigned, usize);
define_big_render_number!(signed, i32);
define_big_render_number!(signed, i64);
define_big_render_number!(signed, i128);
define_big_render_number!(signed, isize);

/// Wrapper type for comparing Rc's via their addresses
pub struct RcEq<T: ?Sized>(pub Rc<T>);

impl<T: ?Sized> PartialEq for RcEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ?Sized> Eq for RcEq<T> {}

impl<T: ?Sized> Deref for RcEq<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> Clone for RcEq<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<T> for RcEq<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<I> From<&[I]> for RcEq<[I]> 
where I: Clone,
{
    fn from(value: &[I]) -> Self {
        Self(Rc::from(value))
    }
}

impl<T> RcEq<T> {
    pub fn new(val: T) -> Self {
        Self(Rc::new(val))
    }
}

pub trait ReqwestUrlExt {
    #[allow(clippy::result_unit_err)]
    fn extend_segments<I>(&mut self, segments: I) -> Result<&mut Self, ()>
    where I: IntoIterator,
    I::Item: AsRef<str>;
    #[allow(clippy::result_unit_err)]
    fn join_segments<I>(&self, segments: I) -> Result<Self, ()>
    where I: IntoIterator,
    I::Item: AsRef<str>,
    Self: Sized;
}

impl ReqwestUrlExt for Url {
    fn extend_segments<I>(&mut self, segments: I) -> Result<&mut Self, ()>
        where I: IntoIterator,
        I::Item: AsRef<str>,
    {
        {
            let mut path = self.path_segments_mut()?;
            path.extend(segments);
        }
        Ok(self)
    }
    fn join_segments<I>(&self, segments: I) -> Result<Self, ()>
        where I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut url = self.clone();
        url.extend_segments(segments)?;
        Ok(url)
    }
}

pub trait ReqwestResponseExt: Sized {
    #[allow(async_fn_in_trait)]  // this is for local use
    async fn check_status(self) -> Result<Self, ErrorContext>;
}

impl ReqwestResponseExt for reqwest::Response {
    async fn check_status(self) -> Result<Self, ErrorContext> {
        let status = self.status();
        if status.is_success() {
            Ok(self)
        } else {
            let body = self.text().await.with_context(|| format!("The server returned a '{status}' status code"))?;
            match serde_json::from_str::<SerializableError>(&body) {
                Err(..) => bail!("The server returned a '{status}' status code with the following body:\n{body}",),
                Ok(err) => Err(err.context("--- SERVER ERROR STACK FOLLOWS ---").context(format!("The server returned a '{status}' status code")))
            }
        }
    }
}

pub async fn api_request<U,R>(url: U) -> Result<R, ErrorContext>
where 
    U: reqwest::IntoUrl,
    R: serde::de::DeserializeOwned,
{
    REQWEST_CLIENT
        .get(url)
        .header("Accept", "application/json")
        .send().await.context("Failed to send the request")?
        .check_status().await?
        .json().await.context("Failed to deserialize response")
}

pub fn sbb_video_link(vid: &str) -> Url {
    let mut url = SBB_BASE.clone();
    url.extend_segments(&["video", vid]).expect("https://sb.ltn.fi/ should be a valid base");
    url
}

pub fn sbb_userid_link(uid: &str) -> Url {
    let mut url = SBB_BASE.clone();
    url.extend_segments(&["userid", uid]).expect("https://sb.ltn.fi/ should be a valid base");
    url
}

/// Stupid recursive function for counting elements in a `VNode` tree
pub fn html_length(html: &Html) -> usize {
    match html {
        Html::VList(ref list) => list.iter().map(html_length).sum(),
        _ => 1,
    }
}

/// Hashes the input using SHA256 in a loop
pub fn sponsorblock_hash(data: &[u8], times: usize) -> String {
    let mut hash_buffer = Array([0u8; 32]);
    let mut hex_buffer = String::with_capacity(64);
    let mut source_buffer: &[u8] = data;
    let mut hasher = Sha256::new();

    for _ in 0..times {
        hasher.update(source_buffer);
        hasher.finalize_into_reset(&mut hash_buffer);
        hex_buffer.clear();
        for byte in &hash_buffer {
            write!(hex_buffer, "{byte:02x}").unwrap();
        }
        source_buffer = hex_buffer.as_bytes();
    }

    hex_buffer
}
