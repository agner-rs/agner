# Issues

## ~~agner-sup: `fixed::SupSpec` should derive `Clone`~~

## ~~agner-sup: export `fixed::fixed_sup::Message` as `fixed::Message`~~

## ~~agner-sup: the trait `dynamic::ChildSpec<IA, M>` should be `dynamic::ChildSpec<IA, Message = M>`~~

## ~~agner-sup: dynamic sup needs a restart strategy~~

## ~~agner-sup: all-for-one does not restart children properly~~

### Example: a sup for several tcp-acceptors.
The acceptors are configured, therefore it is not known at compile time — need to use a dynamic sup.
There should be an ability to escalate the failure of an interface.

## ~~agner-actors: Registered should take away the actor-id upon registered actor's termination~~

## ~~agner-utils: `Yield` future~~
The future that resolves on its second poll. To be used in tight loops.

## agner-actor: measure poll-duration for the actor's futures
And provide those stats as part of `ActorInfo`

## agner-helm: log something! 