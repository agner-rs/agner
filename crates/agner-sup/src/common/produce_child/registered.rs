use crate::common::produce_child::ProduceChildImpl;
use crate::common::WithRegisteredService;

impl<B, AF, M> WithRegisteredService for ProduceChildImpl<B, AF, M> {
    // FIXME: feature
    fn with_registered_service(self, service: agner_registered::Service) -> Self {
        Self { registered_service: Some(service), ..self }
    }
}
