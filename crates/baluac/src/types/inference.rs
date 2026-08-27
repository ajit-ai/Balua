//! Type inference — Hindley-Milner extended with hardware types (Section 5.1)
//! Handles parametric polymorphism, hardware-parameterised types, dependent types for dimensions.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    F16, F32, F64, F128,
    // hardware tensor types
    Fp16, Bf16, Tf32,
    Bool, Char,
    Qubit,
    Var(usize),
    Generic(String),
    App { name: String, args: Vec<Ty> },
    Ref { is_mut: bool, inner: Box<Ty> },
    Ptr { is_mut: bool, inner: Box<Ty> },
    Tensor { inner: Box<Ty>, shape: Vec<usize> },
}

pub struct InferenceEngine {
    next_var: usize,
    subst: HashMap<usize, Ty>,
}

impl InferenceEngine {
    pub fn new() -> Self { Self { next_var: 0, subst: HashMap::new() } }

    pub fn fresh_var(&mut self) -> Ty { let v = self.next_var; self.next_var += 1; Ty::Var(v) }

    /// Unify two types — returns error on dimension mismatch (dependent types)
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), String> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (a, b) {
            (Ty::Var(v), t) | (t, Ty::Var(v)) => { self.subst.insert(v, t); Ok(()) }
            (Ty::Tensor { inner: ai, shape: ash }, Ty::Tensor { inner: bi, shape: bsh }) => {
                if ash != bsh {
                    return Err(format!("Dimension mismatch: {:?} vs {:?} — compile-time error (dependent type)", ash, bsh));
                }
                self.unify(&ai, &bi)
            }
            (x, y) if x == y => Ok(()),
            (x, y) => Err(format!("Type mismatch: {:?} vs {:?}", x, y)),
        }
    }

    fn resolve(&self, ty: &Ty) -> Ty {
        if let Ty::Var(v) = ty { if let Some(t) = self.subst.get(v) { return self.resolve(t); } }
        ty.clone()
    }
}
