//! Structured diagnostics for IDE integration (JSON output)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub span: Option<Span>,
    pub hint: Option<String>,
    /// Hardware context, e.g. "in @hw::gpu kernel"
    pub hardware_context: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: None,
            message: message.into(),
            span: None,
            hint: None,
            hardware_context: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_hardware_context(mut self, ctx: impl Into<String>) -> Self {
        self.hardware_context = Some(ctx.into());
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = &self.span {
            write!(f, "{}:{}:{}: ", span.file, span.line, span.col)?;
        }
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        write!(f, "{}: {}", sev, self.message)?;
        if let Some(hw) = &self.hardware_context {
            write!(f, " [{}]", hw)?;
        }
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    Lex,
    Parse,
    Semantic,
    Codegen,
    Link,
    Opt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileEvent {
    pub kind: EventKind,
    pub duration_ms: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub events: Vec<CompileEvent>,
    pub total_ms: u64,
}

impl Profile {
    pub fn new() -> Self { Self { events: vec![], total_ms: 0 } }
    pub fn record(&mut self, kind: EventKind, duration_ms: u64, detail: impl Into<String>) {
        self.events.push(CompileEvent { kind, duration_ms, detail: detail.into() });
    }
}
