-module(m03).

-export([
        reply_at_once/0,
        forward/1,
        % start_ring/1,
        % call/1,
        run/1
    ]).


start_ring(Size) when is_integer(Size) ->
    Egress = erlang:spawn_link(?MODULE, reply_at_once, []),
    Ingress = lists:foldl(fun(_, Prev) -> 
            erlang:spawn_link(?MODULE, forward, [Prev])
        end, Egress, lists:seq(1, Size)),
    Ingress.



forward(To) ->
    receive
        Payload -> To ! Payload
    end.

reply_at_once() ->
    receive
        {ReplyTo, Ref} when is_pid(ReplyTo) -> ReplyTo ! Ref
    end.

call(Ingress) ->
    Ref = erlang:make_ref(),
    Ingress ! {self(), Ref},
    receive
        Ref -> ok
    end.

run(RingSize) ->
    {SpawnTime, Ingress} = timer:tc(fun() -> start_ring(RingSize) end),
    {CallTime, ok} = timer:tc(fun() -> call(Ingress) end),
    [
        {spawn, SpawnTime},
        {call, CallTime}
    ].

