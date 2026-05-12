use move_trace_format::{format::TraceStack, interface::Tracer};

pub mod concolic;
pub mod fuzz;
pub mod op;
pub mod oracle;
pub mod tree;

#[derive(Default)]
pub struct NopTracer;

impl Tracer for NopTracer {
    fn notify(
        &mut self,
        _event: &move_trace_format::format::TraceEvent,
        _writer: &mut move_trace_format::interface::Writer<'_>,
        _stack: Option<&TraceStack>,
    ) -> bool {
        true
    }
}

pub enum SelectiveTracer<T1, T2> {
    T1(T1),
    T2(T2),
}

impl<T1, T2> Tracer for SelectiveTracer<T1, T2>
where
    T1: Tracer,
    T2: Tracer,
{
    fn notify(
        &mut self,
        event: &move_trace_format::format::TraceEvent,
        writer: &mut move_trace_format::interface::Writer<'_>,
        stack: Option<&TraceStack>,
    ) -> bool {
        match self {
            Self::T1(t) => t.notify(event, writer, stack),
            Self::T2(t) => t.notify(event, writer, stack),
        }
    }
}

#[derive(Default)]
pub struct MayEnableTracer<T> {
    pub tracer: Option<T>,
}

impl<T> MayEnableTracer<T> {
    pub fn new(tracer: T) -> Self {
        Self {
            tracer: Some(tracer),
        }
    }
}

impl<T> Tracer for MayEnableTracer<T>
where
    T: Tracer,
{
    fn notify(
        &mut self,
        event: &move_trace_format::format::TraceEvent,
        writer: &mut move_trace_format::interface::Writer<'_>,
        stack: Option<&TraceStack>,
    ) -> bool {
        if let Some(tracer) = &mut self.tracer {
            tracer.notify(event, writer, stack)
        } else {
            true
        }
    }
}

pub struct CombinedTracer<T1, T2> {
    pub t1: T1,
    pub t2: T2,
}

impl<T1, T2> Tracer for CombinedTracer<T1, T2>
where
    T1: Tracer,
    T2: Tracer,
{
    fn notify(
        &mut self,
        event: &move_trace_format::format::TraceEvent,
        writer: &mut move_trace_format::interface::Writer<'_>,
        stack: Option<&TraceStack>,
    ) -> bool {
        let keep_t1 = self.t1.notify(event, writer, stack);
        let keep_t2 = self.t2.notify(event, writer, stack);
        keep_t1 || keep_t2
    }
}
