use incidence_core::presence::ValueState;

#[test]
fn numeric_zero_is_present_and_distinct_from_other_states() {
    let present_zero = ValueState::Present(0_u64);
    let absent = ValueState::<u64>::Absent;
    let not_modelled = ValueState::<u64>::NotModelled;

    assert_eq!(present_zero, ValueState::Present(0_u64));
    assert!(!(present_zero == absent));
    assert!(!(present_zero == not_modelled));
    assert!(!(absent == not_modelled));
}
