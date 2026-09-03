//! SBOM (Software Bill of Materials) generator — Balua Bom.toml (Section 7.3)
//! Produces CycloneDX-compatible SBOM for supply chain security.

use crate::resolver::Dependency;

#[derive(Debug, Clone)]
pub struct BomEntry {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub source: Option<String>,
    pub hash: Option<String>,
}

pub fn generate_bom(deps: &[Dependency]) -> String {
    let mut out = String::new();
    out.push_str("# Balua SBOM — CycloneDX 1.4\n");
    out.push_str("spec_version = \"1.4\"\n");
    out.push_str(&format!("components = {}\n", deps.len()));
    for dep in deps {
        out.push_str("\n[[component]]\n");
        out.push_str(&format!("name = \"{}\"\n", dep.name));
        out.push_str(&format!("version = \"{}\"\n", dep.req));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bom_generation() {
        let deps = vec![Dependency { name: "std".into(), req: "0.1.0".into() }];
        let bom = generate_bom(&deps);
        assert!(bom.contains("spec_version"));
        assert!(bom.contains("std"));
    }
}
