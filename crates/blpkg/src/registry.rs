//! blpkg.io registry client (Section 7.1)

use anyhow::Result;

pub fn publish() -> Result<()> {
    println!("Publishing to https://blpkg.io ...");
    println!("(stub) Would POST Balua.toml + tarball to registry API");
    Ok(())
}

pub fn fetch(pkg: &str, version: &str) -> Result<String> {
    Ok(format!("Fetched {}@{} from blpkg.io (stub)", pkg, version))
}
