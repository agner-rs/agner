# Issues

## ~~`fixed::SupSpec` should derive `Clone`~~

## ~~export `fixed::fixed_sup::Message` as `fixed::Message`~~

## the trait `dynamic::ChildSpec<IA, M>` should be `dynamic::ChildSpec<IA, Message = M>`

## dynamic sup needs a restart strategy

### Example: a sup for several tcp-acceptors.
The acceptors are configured, therefore it is not known at compile time — need to use a dynamic sup.
There should be an ability to escalate the failure of an interface.

