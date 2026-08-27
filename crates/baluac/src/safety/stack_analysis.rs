//! Stack usage analysis (Section 10.1-2)
//! Compiler emits worst-case stack usage per function; #[max_stack(512)] enforces limit.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StackInfo { pub func: String, pub usage_bytes: usize, pub limit: Option<usize> }

pub struct StackAnalyzer {
    infos: HashMap<String, StackInfo>,
}

impl StackAnalyzer {
    pub fn new() -> Self { Self { infos: HashMap::new() } }

    pub fn record(&mut self, func: &str, usage: usize, limit: Option<usize>) {
        self.infos.insert(func.to_string(), StackInfo { func: func.to_string(), usage_bytes: usage, limit });
    }

    pub fn verify(&self) -> Vec<String> {
        let mut errs = Vec::new();
        for (func, info) in &self.infos {
            if let Some(limit) = info.limit {
                if info.usage_bytes > limit {
                    errs.push(format!("Stack overflow: '{}' uses {} bytes but limit is {} bytes", func, info.usage_bytes, limit));
                }
            }
        }
        errs
    }

    pub fn report(&self) -> String {
        let mut out = String::from("Stack usage report:\n");
        for info in self.infos.values() {
            out.push_str(&format!("  {}: {} bytes{}\n", info.func, info.usage_bytes, info.limit.map(|l| format!(" / limit {}", l)).unwrap_or_default()));
        }
        out
    }
}
