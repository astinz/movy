use libafl::{
    HasMetadata, HasNamedMetadata,
    corpus::{HasCurrentCorpusId, HasTestcase},
    state::{
        HasCorpus, HasCurrentStageId, HasExecutions, HasImported, HasLastFoundTime,
        HasLastReportTime, HasRand, HasSolutions, Stoppable,
    },
};

use movy_types::{error::MovyError, input::MoveAddress, object::MoveObjectInfo};

use crate::outcome::GlobalOutcome;

pub trait FuzzObjectStore {
    fn get_move_object_info(&self, object: MoveAddress) -> Result<MoveObjectInfo, MovyError>;
}

#[cfg(feature = "sui")]
impl<T> FuzzObjectStore for movy_replay::env::SuiTestingEnv<T>
where
    T: movy_replay::db::ObjectStoreInfo,
{
    fn get_move_object_info(&self, object: MoveAddress) -> Result<MoveObjectInfo, MovyError> {
        self.inner().get_move_object_info(object)
    }
}

pub struct ExtraNonSerdeFuzzState<T> {
    pub global_outcome: Option<GlobalOutcome>,
    pub fuzz_env: T,
}

impl<T> ExtraNonSerdeFuzzState<T> {
    pub fn from_env(fuzz_env: T) -> Self {
        Self {
            global_outcome: None,
            fuzz_env,
        }
    }
}

impl<T> std::fmt::Debug for ExtraNonSerdeFuzzState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtraNonSerdeFuzzState")
            .field("global_outcome", &self.global_outcome)
            .finish_non_exhaustive()
    }
}

impl<T> Default for ExtraNonSerdeFuzzState<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            global_outcome: None,
            fuzz_env: T::default(),
        }
    }
}

impl<T> Clone for ExtraNonSerdeFuzzState<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            global_outcome: self.global_outcome.clone(),
            fuzz_env: self.fuzz_env.clone(),
        }
    }
}

pub trait HasExtraState {
    type ExtraState;
    fn extra_state(&self) -> &Self::ExtraState;
    fn extra_state_mut(&mut self) -> &mut Self::ExtraState;
}

pub trait HasFuzzEnv {
    type Env: FuzzObjectStore;

    fn fuzz_env(&self) -> &Self::Env;
    fn fuzz_env_mut(&mut self) -> &mut Self::Env;
}

impl<S, T> HasFuzzEnv for S
where
    S: HasExtraState<ExtraState = ExtraNonSerdeFuzzState<T>>,
    T: FuzzObjectStore,
{
    type Env = T;

    fn fuzz_env(&self) -> &Self::Env {
        &self.extra_state().fuzz_env
    }

    fn fuzz_env_mut(&mut self) -> &mut Self::Env {
        &mut self.extra_state_mut().fuzz_env
    }
}

pub struct SuperState<S, T> {
    pub state: S,
    pub extra: ExtraNonSerdeFuzzState<T>,
}

impl<S, T> SuperState<S, T> {
    pub fn new(state: S, fuzz_env: T) -> Self {
        Self {
            state,
            extra: ExtraNonSerdeFuzzState::from_env(fuzz_env),
        }
    }

    pub fn new_with_default(state: S) -> Self
    where
        T: Default,
    {
        Self {
            state,
            extra: ExtraNonSerdeFuzzState::default(),
        }
    }
}

impl<S, T> HasExtraState for SuperState<S, T> {
    type ExtraState = ExtraNonSerdeFuzzState<T>;
    fn extra_state(&self) -> &Self::ExtraState {
        &self.extra
    }

    fn extra_state_mut(&mut self) -> &mut Self::ExtraState {
        &mut self.extra
    }
}

impl<S: HasMetadata, T> HasMetadata for SuperState<S, T> {
    fn metadata_map(&self) -> &libafl_bolts::serdeany::SerdeAnyMap {
        self.state.metadata_map()
    }

    fn metadata_map_mut(&mut self) -> &mut libafl_bolts::serdeany::SerdeAnyMap {
        self.state.metadata_map_mut()
    }
}

impl<S: HasNamedMetadata, T> HasNamedMetadata for SuperState<S, T> {
    fn named_metadata_map(&self) -> &libafl_bolts::serdeany::NamedSerdeAnyMap {
        self.state.named_metadata_map()
    }
    fn named_metadata_map_mut(&mut self) -> &mut libafl_bolts::serdeany::NamedSerdeAnyMap {
        self.state.named_metadata_map_mut()
    }
}

impl<S: HasRand, T> HasRand for SuperState<S, T> {
    type Rand = S::Rand;
    fn rand_mut(&mut self) -> &mut Self::Rand {
        self.state.rand_mut()
    }

    fn rand(&self) -> &Self::Rand {
        self.state.rand()
    }
}

impl<S: HasExecutions, T> HasExecutions for SuperState<S, T> {
    fn executions(&self) -> &u64 {
        self.state.executions()
    }

    fn executions_mut(&mut self) -> &mut u64 {
        self.state.executions_mut()
    }
}

impl<S: HasCorpus<I>, I, T> HasCorpus<I> for SuperState<S, T> {
    type Corpus = S::Corpus;

    fn corpus(&self) -> &Self::Corpus {
        self.state.corpus()
    }
    fn corpus_mut(&mut self) -> &mut Self::Corpus {
        self.state.corpus_mut()
    }
}

impl<S: HasSolutions<I>, I, T> HasSolutions<I> for SuperState<S, T> {
    type Solutions = S::Solutions;

    fn solutions(&self) -> &Self::Solutions {
        self.state.solutions()
    }

    fn solutions_mut(&mut self) -> &mut Self::Solutions {
        self.state.solutions_mut()
    }
}

impl<S: HasLastFoundTime, T> HasLastFoundTime for SuperState<S, T> {
    fn last_found_time(&self) -> &std::time::Duration {
        self.state.last_found_time()
    }
    fn last_found_time_mut(&mut self) -> &mut std::time::Duration {
        self.state.last_found_time_mut()
    }
}

impl<S: Stoppable, T> Stoppable for SuperState<S, T> {
    fn stop_requested(&self) -> bool {
        self.state.stop_requested()
    }
    fn discard_stop_request(&mut self) {
        self.state.discard_stop_request();
    }

    fn request_stop(&mut self) {
        self.state.request_stop();
    }
}

impl<S: HasTestcase<I>, I, T> HasTestcase<I> for SuperState<S, T> {
    fn testcase(
        &self,
        id: libafl::corpus::CorpusId,
    ) -> Result<std::cell::Ref<'_, libafl::corpus::Testcase<I>>, libafl::Error> {
        self.state.testcase(id)
    }

    fn testcase_mut(
        &self,
        id: libafl::corpus::CorpusId,
    ) -> Result<std::cell::RefMut<'_, libafl::corpus::Testcase<I>>, libafl::Error> {
        self.state.testcase_mut(id)
    }
}

impl<S: HasCurrentCorpusId, T> HasCurrentCorpusId for SuperState<S, T> {
    fn clear_corpus_id(&mut self) -> Result<(), libafl::Error> {
        self.state.clear_corpus_id()
    }

    fn current_corpus_id(&self) -> Result<Option<libafl::corpus::CorpusId>, libafl::Error> {
        self.state.current_corpus_id()
    }

    fn set_corpus_id(&mut self, id: libafl::corpus::CorpusId) -> Result<(), libafl::Error> {
        self.state.set_corpus_id(id)
    }
}

impl<S: HasLastReportTime, T> HasLastReportTime for SuperState<S, T> {
    fn last_report_time(&self) -> &Option<std::time::Duration> {
        self.state.last_report_time()
    }
    fn last_report_time_mut(&mut self) -> &mut Option<std::time::Duration> {
        self.state.last_report_time_mut()
    }
}

impl<S: HasImported, T> HasImported for SuperState<S, T> {
    fn imported(&self) -> &usize {
        self.state.imported()
    }
    fn imported_mut(&mut self) -> &mut usize {
        self.state.imported_mut()
    }
}

impl<S: HasCurrentStageId, T> HasCurrentStageId for SuperState<S, T> {
    fn clear_stage_id(&mut self) -> Result<(), libafl::Error> {
        self.state.clear_stage_id()
    }
    fn on_restart(&mut self) -> Result<(), libafl::Error> {
        self.state.on_restart()
    }
    fn current_stage_id(&self) -> Result<Option<libafl::stages::StageId>, libafl::Error> {
        self.state.current_stage_id()
    }
    fn set_current_stage_id(&mut self, id: libafl::stages::StageId) -> Result<(), libafl::Error> {
        self.state.set_current_stage_id(id)
    }
}
