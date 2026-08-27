//! Hardware-parameterised types & type classes (Section 5.1-6)
//! GpuBackend, FpgaTarget, QuantumBackend traits; GpuTensor<T,Shape,Backend> etc.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareTypeClass { GpuBackend, FpgaTarget, QuantumBackend }

#[derive(Debug, Clone)]
pub struct TypeClassImpl { pub class: HardwareTypeClass, pub for_ty: String }

pub struct HardwareTypeRegistry {
    impls: HashMap<String, TypeClassImpl>,
}

impl HardwareTypeRegistry {
    pub fn new() -> Self { Self { impls: HashMap::new() } }

    pub fn register(&mut self, ty: &str, class: HardwareTypeClass) {
        self.impls.insert(ty.to_string(), TypeClassImpl { class, for_ty: ty.to_string() });
    }

    pub fn check(&self, ty: &str, expected: HardwareTypeClass) -> Result<(), String> {
        match self.impls.get(ty) {
            Some(imp) if imp.class == expected => Ok(()),
            Some(imp) => Err(format!("Type '{}' implements {:?} but required {:?}", ty, imp.class, expected)),
            None => Err(format!("Type '{}' does not implement {:?}", ty, expected)),
        }
    }
}
