use crate::common::produce_child::ProduceChildImpl;
use crate::common::WithRegisteredService;

impl<B, AF, M> WithRegisteredService for ProduceChildImpl<B, AF, M> {
    fn with_registered_service(self, service: agner_reg::Service) -> Self {
        Self { registered_service: Some(service), ..self }
    }
}
