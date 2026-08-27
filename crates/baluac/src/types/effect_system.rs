//! Effect system — IO + HardwareIO, Pure, Unsafe (Section 5.1-7)
//! fn read_sensor() -> i32 / IO + HardwareIO ; fn pure_math(x: f64) -> f64 / Pure

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect { Pure, IO, HardwareIO, Unsafe, Async }

pub struct EffectChecker {
    func_effects: std::collections::HashMap<String, Vec<Effect>>,
}

impl EffectChecker {
    pub fn new() -> Self { Self { func_effects: std::collections::HashMap::new() } }

    pub fn declare(&mut self, func: &str, effects: Vec<Effect>) {
        self.func_effects.insert(func.to_string(), effects);
    }

    /// Validate that caller effects subsume callee effects (unsafe must be marked)
    pub fn check_call(&self, caller: &str, callee: &str) -> Result<(), String> {
        let caller_effs = self.func_effects.get(caller).cloned().unwrap_or(vec![Effect::Pure]);
        let callee_effs = self.func_effects.get(callee).cloned().unwrap_or(vec![Effect::Pure]);
        for eff in &callee_effs {
            if *eff == Effect::Unsafe && !caller_effs.contains(&Effect::Unsafe) {
                return Err(format!("Function '{}' calls unsafe '{}' without Unsafe effect", caller, callee));
            }
            if *eff == Effect::IO && !caller_effs.contains(&Effect::IO) && !caller_effs.contains(&Effect::Unsafe) {
                return Err(format!("Function '{}' calls IO '{}' without IO effect", caller, callee));
            }
        }
        Ok(())
    }
}
