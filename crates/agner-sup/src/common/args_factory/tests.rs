use crate::common::args_factory;

#[test]
fn ergonomics() {
    let mut af_clone = args_factory::clone(vec![1, 2, 3, 4]);
    let mut af_lazy = args_factory::call(|| vec![1, 2, 3, 4]);
    let mut af_call = args_factory::map({
        let mut acc = vec![];
        move |v: usize| -> usize {
            acc.push(v);
            acc.iter().copied().sum()
        }
    });

    assert_eq!(af_clone.make_args(()), &[1, 2, 3, 4]);
    assert_eq!(af_lazy.make_args(()), &[1, 2, 3, 4]);
    assert_eq!(af_call.make_args(1), 1);
    assert_eq!(af_call.make_args(2), 3);
    assert_eq!(af_call.make_args(3), 6);
}
