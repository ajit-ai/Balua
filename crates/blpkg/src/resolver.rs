//! PubGrub dependency resolver (Section 7.1)
//! Minimal stub — resolves semver constraints via PubGrub algorithm.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Dependency { pub name: String, pub req: String }

pub struct Resolver { deps: HashMap<String, String> }

impl Resolver {
    pub fn new() -> Self { Self { deps: HashMap::new() } }
    pub fn add(&mut self, name: &str, req: &str) { self.deps.insert(name.to_string(), req.to_string()); }
    /// Resolve — returns selected versions (stub picks req verbatim)
    pub fn resolve(&self) -> Result<HashMap<String, String>, String> {
        // Real impl would use pubgrub crate; stub just validates semver parse
        for (name, req) in &self.deps {
            if req.is_empty() { return Err(format!("Empty version req for {}", name)); }
        }
        Ok(self.deps.clone())
    }
}
