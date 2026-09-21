//! PubGrub dependency resolver (Section 7.1)
//! M1 local-only: validates semver constraints with the `semver` crate.
//! No network access; registry resolution is post-GA.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Dependency { pub name: String, pub req: String }

pub struct Resolver { deps: HashMap<String, String> }

impl Resolver {
    pub fn new() -> Self { Self { deps: HashMap::new() } }
    pub fn add(&mut self, name: &str, req: &str) { self.deps.insert(name.to_string(), req.to_string()); }
    /// Resolve — validates each requirement parses as semver, returns the map.
    /// Post-GA this becomes real PubGrub resolution against blpkg.io.
    pub fn resolve(&self) -> Result<HashMap<String, String>, String> {
        for (name, req) in &self.deps {
            if req.is_empty() {
                return Err(format!("Empty version req for {}", name));
            }
            semver::VersionReq::parse(req)
                .map_err(|e| format!("Invalid semver req '{}' for {}: {}", req, name, e))?;
        }
        Ok(self.deps.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_req() {
        let mut r = Resolver::new();
        r.add("balua-tensor", "^1.2.0");
        assert!(r.resolve().is_ok());
    }

    #[test]
    fn rejects_empty_and_invalid_req() {
        let mut r = Resolver::new();
        r.add("bad", "");
        assert!(r.resolve().is_err());
        let mut r2 = Resolver::new();
        r2.add("bad", "not-a-version!!!");
        assert!(r2.resolve().is_err());
    }
}
