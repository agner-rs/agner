pub trait WithRegisteredService: Sized {
    // FIXME: feature
    fn with_registered_service(self, service: agner_registered::Service) -> Self;
}
