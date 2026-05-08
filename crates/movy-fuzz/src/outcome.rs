use std::{collections::BTreeMap, fmt::Display};

use libafl::executors::ExitKind;
use movy_types::{
    input::FunctionIdent,
    oracle::{Event, OracleFinding},
};
use serde::{Deserialize, Serialize};

use crate::trace::Log;

#[cfg(feature = "sui")]
pub type SolverState = movy_replay::tracer::concolic::ConcolicState;

#[cfg(not(feature = "sui"))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SolverState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionExtraOutcome {
    pub logs: BTreeMap<FunctionIdent, Vec<Log>>,
    pub solver: SolverState,
    pub stage_idx: Option<usize>,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub events_verdict: ExitKind,
    pub events: Vec<Event>,
    #[serde(default)]
    pub allowed_success: bool,
    #[serde(default)]
    pub findings: Vec<OracleFinding>,
}

impl Display for ExecutionOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExecutionOutcome {{ events_verdict: {:?}, allowed_success: {}, findings: {:?} }}",
            self.events_verdict, self.allowed_success, self.findings
        )
    }
}

#[derive(Debug, Clone)]
pub struct GlobalOutcome {
    pub exec: ExecutionOutcome,
    pub extra: ExecutionExtraOutcome,
}
