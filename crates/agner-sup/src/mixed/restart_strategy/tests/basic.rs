use super::*;

#[test]
fn one_for_one_basic_test() {
    let sup = next_id();

    let mut decider = TestDecider::new(sup, RestartType::One, RestartIntensity::new(3, 60));

    assert!(decider.next_action().unwrap().is_none());

    assert!(decider.add_child("first", ChildType::Permanent).is_ok());
    assert!(decider.add_child("second", ChildType::Permanent).is_ok());
    assert!(decider.add_child("third", ChildType::Permanent).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);

    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);

    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!( &action, Action::Shutdown(reason) if reason.is_shutdown() ), "{:?}", action);
    assert!(decider.expected_exits().is_empty());
}

#[test]
fn all_for_one_basic_test() {
    let sup = next_id();

    let mut decider = TestDecider::new(sup, RestartType::All, RestartIntensity::new(3, 60));

    assert!(decider.next_action().unwrap().is_none());

    assert!(decider.add_child("first", ChildType::Permanent).is_ok());
    assert!(decider.add_child("second", ChildType::Permanent).is_ok());
    assert!(decider.add_child("third", ChildType::Permanent).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);
    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);
    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);
    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);
    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!( &action, Action::Shutdown(reason) if reason.is_shutdown() ), "{:?}", action);
    assert!(decider.expected_exits().is_empty());
}

#[test]
fn rest_for_one_basic_test() {
    let sup = next_id();

    let mut decider = TestDecider::new(sup, RestartType::Rest, RestartIntensity::new(3, 60));

    assert!(decider.next_action().unwrap().is_none());

    assert!(decider.add_child("first", ChildType::Permanent).is_ok());
    assert!(decider.add_child("second", ChildType::Permanent).is_ok());
    assert!(decider.add_child("third", ChildType::Permanent).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("first")), "{:?}", action);

    let first = next_id();
    assert!(decider.child_started("first", first).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("second")), "{:?}", action);

    let second = next_id();
    assert!(decider.child_started("second", second).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Start("third")), "{:?}", action);

    let third = next_id();
    assert!(decider.child_started("third", third).is_ok());

    assert!(decider.next_action().unwrap().is_none());
    assert!(decider.expected_exits().is_empty());

    // crash
    assert!(decider.exit_signal(second, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("third")), "{:?}", action);
    assert!(decider.exit_signal(third, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!(&action, Action::Stop("first")), "{:?}", action);
    assert!(decider.exit_signal(first, Exit::shutdown(), next_tick()).is_ok());

    let action = decider.next_action().unwrap().unwrap();
    assert!(matches!( &action, Action::Shutdown(reason) if reason.is_shutdown() ), "{:?}", action);
    assert!(decider.expected_exits().is_empty());
}
