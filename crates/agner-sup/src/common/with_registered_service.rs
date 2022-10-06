pub trait WithRegisteredService: Sized {
    fn with_registered_service(self, service: agner_reg::Service) -> Self;
}
