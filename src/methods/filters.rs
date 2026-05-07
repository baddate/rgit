// sorry clippy, we don't have a choice. askama forces this on us
#![allow(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]

use std::{
    borrow::Borrow,
    fmt::Display,
    sync::{Arc, LazyLock},
};

use arc_swap::ArcSwap;
use rkyv::{
    rend::{i32_le, i64_le},
    tuple::ArchivedTuple2,
};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

// pub fn format_time(s: impl Borrow<time::OffsetDateTime>) -> Result<String, askama::Error> {
pub fn format_time(s: impl Into<Timestamp>) -> Result<String, askama::Error> {
    let s = s.into().0;

    (*s.borrow())
        .format(&Rfc3339)
        .map_err(Box::from)
        .map_err(askama::Error::Custom)
}

pub fn branch_query(branch: Option<&str>) -> String {
    if let Some(b) = branch {
        format!("?h={b}")
    } else {
        String::new()
    }
}

pub fn timeago(s: impl Into<Timestamp>) -> Result<String, askama::Error> {
    Ok(timeago::Formatter::new()
        .convert((OffsetDateTime::now_utc() - s.into().0).try_into().unwrap()))
}

pub fn age_class(s: impl Into<Timestamp>) -> Result<&'static str, askama::Error> {
    let elapsed = (OffsetDateTime::now_utc() - s.into().0)
        .try_into()
        .unwrap_or(std::time::Duration::MAX);
    let secs = elapsed.as_secs();
    Ok(if secs < 86_400 {
        "age-hours"
    } else if secs < 7 * 86_400 {
        "age-days"
    } else if secs < 30 * 86_400 {
        "age-weeks"
    } else if secs < 365 * 86_400 {
        "age-months"
    } else {
        "age-years"
    })
}

pub fn file_perms(s: u16) -> Result<String, askama::Error> {
    Ok(unix_mode::to_string(u32::from(s)))
}

pub struct DisplayHexBuffer<const N: usize>(pub const_hex::Buffer<N>);

impl<const N: usize> Display for DisplayHexBuffer<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

pub fn hex(s: &[u8; 20]) -> Result<DisplayHexBuffer<20>, askama::Error> {
    let mut buf = const_hex::Buffer::new();
    buf.format(s);
    Ok(DisplayHexBuffer(buf))
}

pub fn gravatar(email: &str) -> Result<&'static str, askama::Error> {
    static CACHE: LazyLock<ArcSwap<hashbrown::HashMap<&'static str, &'static str>>> =
        LazyLock::new(|| ArcSwap::new(Arc::new(hashbrown::HashMap::new())));

    if let Some(res) = CACHE.load().get(email).copied() {
        return Ok(res);
    }

    let url = libravatar_url(email);
    let key = Box::leak(Box::from(email));
    let url = url.leak();

    CACHE.rcu(|curr| {
        let mut r = (**curr).clone();
        r.insert(key, url);
        r
    });

    Ok(url)
}

fn libravatar_url(email: &str) -> String {
    use sha2::{Digest, Sha256};

    let hash = const_hex::encode(Sha256::digest(email.trim().to_lowercase().as_bytes()));
    format!("https://seccdn.libravatar.org/avatar/{hash}")
}

pub struct Timestamp(OffsetDateTime);

impl From<&ArchivedTuple2<i64_le, i32_le>> for Timestamp {
    fn from(value: &ArchivedTuple2<i64_le, i32_le>) -> Self {
        Self(
            OffsetDateTime::from_unix_timestamp(value.0.to_native())
                .unwrap()
                .to_offset(UtcOffset::from_whole_seconds(value.1.to_native()).unwrap()),
        )
    }
}

impl From<(i64, i32)> for Timestamp {
    fn from(value: (i64, i32)) -> Self {
        Self(
            OffsetDateTime::from_unix_timestamp(value.0)
                .unwrap()
                .to_offset(UtcOffset::from_whole_seconds(value.1).unwrap()),
        )
    }
}

impl From<&(i64, i32)> for Timestamp {
    fn from(value: &(i64, i32)) -> Self {
        Self(
            OffsetDateTime::from_unix_timestamp(value.0)
                .unwrap()
                .to_offset(UtcOffset::from_whole_seconds(value.1).unwrap()),
        )
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(value: OffsetDateTime) -> Self {
        Self(value)
    }
}
