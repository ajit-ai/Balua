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

/// Full SBOM from `BomEntry` values, including license/source/hash when set.
/// Used by `blpkg bom` post-GA (E6); covered by unit tests in M1.
#[allow(dead_code)]
pub fn generate_bom_full(entries: &[BomEntry]) -> String {
    let mut out = String::new();
    out.push_str("# Balua SBOM — CycloneDX 1.4\n");
    out.push_str("spec_version = \"1.4\"\n");
    out.push_str(&format!("components = {}\n", entries.len()));
    for e in entries {
        out.push_str("\n[[component]]\n");
        out.push_str(&format!("name = \"{}\"\n", e.name));
        out.push_str(&format!("version = \"{}\"\n", e.version));
        if let Some(lic) = &e.license {
            out.push_str(&format!("license = \"{}\"\n", lic));
        }
        if let Some(src) = &e.source {
            out.push_str(&format!("source = \"{}\"\n", src));
        }
        if let Some(hash) = &e.hash {
            out.push_str(&format!("hash = \"{}\"\n", hash));
        }
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

    #[test]
    fn bom_full_emits_provenance_fields() {
        let entries = vec![BomEntry { name: "std".into(), version: "0.1.0".into(), license: Some("MIT".into()), source: Some("local".into()), hash: Some("sha256:abc".into()) }];
        let bom = generate_bom_full(&entries);
        assert!(bom.contains("license = \"MIT\""));
        assert!(bom.contains("source = \"local\""));
        assert!(bom.contains("hash = \"sha256:abc\""));
    }
}
