//! Formal verification hooks — Frama-C / CBMC (Section 10.1-5)
//! #[requires(x > 0)] #[ensures(result > x)] → emit verification conditions.

#[derive(Debug, Clone)]
pub struct Contract {
    pub func: String,
    pub requires: Vec<String>,
    pub ensures: Vec<String>,
}

pub struct VerificationEmitter {
    contracts: Vec<Contract>,
}

impl VerificationEmitter {
    pub fn new() -> Self { Self { contracts: vec![] } }

    pub fn add_contract(&mut self, func: &str, requires: Vec<String>, ensures: Vec<String>) {
        self.contracts.push(Contract { func: func.to_string(), requires, ensures });
    }

    pub fn emit_frama_c(&self) -> String {
        let mut out = String::from("/* Balua formal verification — Frama-C ACSL */\n");
        for c in &self.contracts {
            for req in &c.requires { out.push_str(&format!("/*@ requires {}; */\n", req)); }
            for ens in &c.ensures { out.push_str(&format!("/*@ ensures {}; */\n", ens)); }
            out.push_str(&format!("int {}();\n\n", c.func));
        }
        out
    }

    pub fn emit_cbmc(&self) -> String {
        let mut out = String::from("// Balua CBMC harness\n#include <assert.h>\n");
        for c in &self.contracts {
            out.push_str(&format!("\nvoid harness_{}() {{\n", c.func));
            for req in &c.requires { out.push_str(&format!("  __CPROVER_assume({});\n", req)); }
            out.push_str(&format!("  int result = {}();\n", c.func));
            for ens in &c.ensures { out.push_str(&format!("  __CPROVER_assert({}, \"{}\");\n", ens, ens)); }
            out.push_str("}\n");
        }
        out
    }
}
