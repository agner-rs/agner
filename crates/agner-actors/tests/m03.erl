-module(m03).

-export([
        reply_at_once/0,
        forward/1,
        % start_ring/1,
        % call/1,
        run/2
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
    end,
    forward(To).

reply_at_once() ->
    receive
        {ReplyTo, Ref} when is_pid(ReplyTo) -> ReplyTo ! Ref
    end,
    reply_at_once().

call(Ingress) ->
    Ref = erlang:make_ref(),
    Ingress ! {self(), Ref},
    receive
        Ref -> ok
    end.

run(RingSize, Times) ->
    {SpawnTime, Ingress} = timer:tc(fun() -> start_ring(RingSize) end),
    io:format("spawn-time: ~p~n", [SpawnTime]),

    lists:foreach(fun(I) ->
        {CallTime, ok} = timer:tc(fun() -> call(Ingress) end),
        io:format("[~p] RTT: ~p~n", [I, CallTime])
    end, lists:seq(1, Times)).

