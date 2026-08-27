//! WCET analysis — Worst-Case Execution Time (Section 10.1-3)
//! #[wcet_cycles(1000)] — compiler verifies bound statically where possible.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WcetAnnotation { pub func: String, pub max_cycles: u64 }

pub struct WcetAnalyzer {
    annotations: HashMap<String, u64>,
    estimated: HashMap<String, u64>,
}

impl WcetAnalyzer {
    pub fn new() -> Self { Self { annotations: HashMap::new(), estimated: HashMap::new() } }

    pub fn annotate(&mut self, func: &str, cycles: u64) { self.annotations.insert(func.to_string(), cycles); }

    pub fn estimate(&mut self, func: &str, cycles: u64) { self.estimated.insert(func.to_string(), cycles); }

    pub fn verify(&self) -> Vec<String> {
        let mut errs = Vec::new();
        for (func, max) in &self.annotations {
            if let Some(est) = self.estimated.get(func) {
                if est > max {
                    errs.push(format!("WCET violation: '{}' estimated {} cycles exceeds bound {} cycles", func, est, max));
                }
            }
        }
        errs
    }
}
