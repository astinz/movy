use std::{borrow::Cow, marker::PhantomData, ops::AddAssign};

use libafl::{
    HasMetadata,
    executors::{Executor, ExitKind, HasObservers},
    observers::{MapObserver, ObserversTuple, StdMapObserver},
    state::{HasExecutions, HasRand},
};
use libafl_bolts::tuples::{Handle, MatchNameRef, RefIndexable};
use log::trace;
use movy_replay::{
    db::{ObjectStoreInfo, ObjectStoreMintObject},
    exec::{ExecutionTracedResults, SuiExecutor},
    tracer::{fuzz::SuiFuzzTracer, oracle::SuiGeneralOracle},
};
use movy_sui::database::cache::{CachedStore, ObjectSuiStoreCommit};
use movy_types::input::MoveAddress;
use sui_types::{
    effects::TransactionEffectsAPI,
    execution_status::ExecutionStatus,
    storage::{BackingStore, ObjectStore},
};

use crate::{
    input::MoveInput,
    meta::HasFuzzMetadata,
    operations::fuzz::CODE_OBSERVER_NAME,
    outcome::{ExecutionExtraOutcome, ExecutionOutcome, GlobalOutcome},
    state::{ExtraNonSerdeFuzzState, HasExtraState, HasFuzzEnv},
};

pub struct SuiFuzzExecutor<T, OT, RT, I, S> {
    pub executor: SuiExecutor<T>,
    pub ob: OT,
    pub attacker: MoveAddress,
    pub oracles: RT,
    // pub minted_gas: Object,
    // pub log_tracer: Option<SuiLogTracer>,
    pub ph: PhantomData<(I, S)>,
    pub epoch: u64,
    pub epoch_ms: u64,
}

impl<T, OT, RT, I, S> HasObservers for SuiFuzzExecutor<T, OT, RT, I, S> {
    type Observers = OT;
    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        RefIndexable::from(&self.ob)
    }
    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
        RefIndexable::from(&mut self.ob)
    }
}

impl<EM, Z, T, OT, RT, I, S, E> Executor<EM, I, S, Z> for SuiFuzzExecutor<T, OT, RT, I, S>
where
    T: ObjectStore + BackingStore + ObjectSuiStoreCommit + ObjectStoreMintObject + ObjectStoreInfo,
    OT: ObserversTuple<I, S>,
    RT: for<'a> SuiGeneralOracle<CachedStore<&'a T>, S>,
    I: MoveInput,
    S: HasRand
        + HasFuzzMetadata
        + HasExecutions
        + HasMetadata
        + HasFuzzEnv<Env = E>
        + HasExtraState<ExtraState = ExtraNonSerdeFuzzState<E>>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut S,
        _mgr: &mut EM,
        input: &I,
    ) -> Result<ExitKind, libafl::Error> {
        // Clear any pending outcome
        // state.extra_state_mut().extra = None;
        let epoch = state.fuzz_state().epoch;
        let epoch_ms = state.fuzz_state().epoch_ms;
        {
            let code_ob: &mut StdMapObserver<'_, u8, false> = self
                .ob
                .get_mut(&Handle::new(Cow::Borrowed(CODE_OBSERVER_NAME)))
                .expect("no code ob installed");
            code_ob[0] = 1;
        }

        let db = CachedStore::new(&self.executor.db);
        self.oracles.pre_execution(&db, state, input.sequence())?;

        trace!("Executing input: {}", input.sequence());
        state.executions_mut().add_assign(1);
        let gas_id = state.fuzz_state().gas_id;
        let tracer = SuiFuzzTracer::new(&mut self.ob, state, &mut self.oracles, CODE_OBSERVER_NAME);

        let result = self.executor.run_ptb_with_gas(
            input.sequence().to_ptb()?,
            epoch,
            epoch_ms,
            self.attacker.into(),
            gas_id.into(),
            Some(tracer),
        )?;

        let ExecutionTracedResults { results, tracer } = result;
        let effects = results.effects;
        let events = results.store.events.data.clone();
        db.commit_store(results.store, &effects)
            .map_err(|e| libafl::Error::unknown(format!("commit store failed: {e}")))?;

        let mut trace_outcome = tracer
            .expect("tracer should be present when tracing is enabled")
            .outcome();

        trace!("Execution finished with status: {:?}", effects.status());

        let (stage_idx, success) = match effects.status() {
            ExecutionStatus::Failure(failure) => (
                // command index may be out of bound when meeting non-aborted error
                if failure
                    .command
                    .is_some_and(|c| c < input.sequence().commands.len())
                {
                    failure.command
                } else {
                    None
                },
                false,
            ),
            _ => (None, true),
        };
        if effects.status().is_err() {
            let code_ob: &mut StdMapObserver<'_, u8, false> = self
                .ob
                .get_mut(&Handle::new(Cow::Borrowed(CODE_OBSERVER_NAME)))
                .expect("no code ob installed");
            code_ob.reset_map()?;
        }
        let extra = ExecutionExtraOutcome {
            logs: trace_outcome.logs,
            solver: trace_outcome.concolic,
            stage_idx,
            success,
        };

        if log::log_enabled!(log::Level::Debug) {
            for ev in events.iter() {
                if let Some((st, ev)) = state.fuzz_state().decode_sui_event(ev)? {
                    log::debug!(
                        "Event: {}({})",
                        st.to_canonical_string(true),
                        serde_json::to_string(&ev)
                            .unwrap_or_else(|e| format!("json err({}): {:?}", e, ev))
                    );
                } else {
                    log::debug!(
                        "Event {} missing for decoding",
                        ev.type_.to_canonical_string(true)
                    );
                }
            }
        }
        let events: Vec<_> = events.into_iter().map(|e| e.into()).collect();

        // Expose preliminary outcome so oracles can inspect events.
        let mut exec = ExecutionOutcome {
            events_verdict: trace_outcome.verdict,
            events: events.clone(),
            allowed_success: success,
            findings: trace_outcome.findings.clone(),
        };
        state.extra_state_mut().global_outcome = Some(GlobalOutcome {
            exec: exec.clone(),
            extra: extra.clone(),
        });

        let oracle_vulns = self.oracles.done_execution(&db, state, &effects)?;
        if !oracle_vulns.is_empty() {
            trace_outcome.findings.extend(oracle_vulns.iter().cloned());
        }
        let kind = if !oracle_vulns.is_empty() {
            ExitKind::Crash
        } else {
            trace_outcome.verdict
        };
        exec = ExecutionOutcome {
            events_verdict: kind,
            events,
            allowed_success: success,
            findings: trace_outcome.findings.clone(),
        };
        state.extra_state_mut().global_outcome = Some(GlobalOutcome { exec, extra });

        // if let Some(tracer) = &self.log_tracer
        //     && let Some(v) = tracer.may_log(input, &outcome, &extra)
        // {
        //     info!("{}", v);
        // }

        Ok(kind)
    }
}
