use std::{rc::Rc, ops::Deref};

use anyhow::Context;
use chrono::{DateTime, Utc, NaiveDateTime};
use reqwest::Url;
use serde::Deserialize;

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


#[derive(Deserialize)]
struct OEmbedResponse {
    title: Option<String>,
}

pub async fn get_original_title(vid: String) -> Result<String, anyhow::Error> {
    let url = Url::parse_with_params(
        "https://www.youtube-nocookie.com/oembed", 
        &[("url", format!("https://youtu.be/{vid}"))]
    ).context("Failed to construct an oembed request URL")?;
    let resp: OEmbedResponse = reqwest::get(url).await.context("Failed to send oembed request")?
        .json().await.context("Failed to deserialize oembed response")?;
    resp.title.context("oembed response contained no title")
}

/// Wrapper type for comparing Rc's via their addresses
#[derive(Clone)]
pub struct RcEq<T>(pub Rc<T>);

impl<T> PartialEq for RcEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Deref for RcEq<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}
