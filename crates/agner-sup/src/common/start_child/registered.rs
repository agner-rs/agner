use crate::common::start_child::StartChildImpl;
use crate::common::WithRegisteredService;

impl<B, A, M> WithRegisteredService for StartChildImpl<B, A, M> {
    // FIXME: feature
    fn with_registered_service(mut self, service: agner_reg::Service) -> Self {
        Self { registered_service: Some(service), ..self }
    }
}
