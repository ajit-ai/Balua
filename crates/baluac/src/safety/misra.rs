//! MISRA / AUTOSAR / DO-178C / IEC61508 safety profiles (Section 10.1)
//! Disables dynamic allocation, recursion, unbounded loops when profile active.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyProfile { AutosarCpp14, Do178C, Iec61508, None }

pub struct SafetyChecker { pub profile: SafetyProfile }

impl SafetyChecker {
    pub fn new(profile: SafetyProfile) -> Self { Self { profile } }

    pub fn check_function(&self, name: &str, uses_heap: bool, is_recursive: bool, has_unbounded_loop: bool) -> Vec<String> {
        if self.profile == SafetyProfile::None { return vec![]; }
        let mut errs = Vec::new();
        if uses_heap { errs.push(format!("[{:?}] Function '{}' uses heap allocation — forbidden", self.profile, name)); }
        if is_recursive { errs.push(format!("[{:?}] Function '{}' is recursive — forbidden", self.profile, name)); }
        if has_unbounded_loop { errs.push(format!("[{:?}] Function '{}' has unbounded loop — forbidden", self.profile, name)); }
        errs
    }
}
