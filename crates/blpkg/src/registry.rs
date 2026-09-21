//! blpkg.io registry client (Section 7.1)
//! M1 local-only: no network. Publish/fetch are explicit post-GA errors.

use anyhow::Result;

pub fn publish() -> Result<()> {
    anyhow::bail!("blpkg.io registry is post-GA (E6) — publish unavailable in local-only M1")
}

#[allow(dead_code)]
pub fn fetch(pkg: &str, version: &str) -> Result<String> {
    anyhow::bail!("blpkg.io registry is post-GA (E6) — cannot fetch {}@{} locally", pkg, version)
}
