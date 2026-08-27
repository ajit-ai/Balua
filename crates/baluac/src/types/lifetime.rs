//! Lifetime analysis — cross-device lifetimes (Section 5.1-5)
//! e.g., fn gpu_ref<'dev>(buf: &'dev DeviceBuf<f32>) -> GpuSlice<'dev, f32>

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime(pub String);

#[derive(Debug)]
pub struct LifetimeEnv {
    scopes: Vec<HashMap<String, Lifetime>>,
}

impl LifetimeEnv {
    pub fn new() -> Self { Self { scopes: vec![HashMap::new()] } }
    pub fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    pub fn pop_scope(&mut self) { self.scopes.pop(); }
    pub fn insert(&mut self, var: &str, lt: Lifetime) { if let Some(s) = self.scopes.last_mut() { s.insert(var.to_string(), lt); } }
    pub fn get(&self, var: &str) -> Option<Lifetime> { for s in self.scopes.iter().rev() { if let Some(lt) = s.get(var) { return Some(lt.clone()); } } None }

    /// Validate that device lifetime outlives slice borrow
    pub fn validate_device_lifetime(&self, buf_lt: &Lifetime, slice_lt: &Lifetime) -> Result<(), String> {
        if buf_lt != slice_lt {
            return Err(format!("Lifetime mismatch: buffer lives '{:?} but slice requires '{:?} — cross-device borrow invalid.", buf_lt, slice_lt));
        }
        Ok(())
    }
}
