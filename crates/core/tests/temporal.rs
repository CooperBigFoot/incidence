use myproject_core::temporal::{
    CalendarInstant, CalendarOrigin, FixedStepCalendar, RunHorizon, TemporalError,
    TimestepDuration, TimestepIndex,
};

fn assert_ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected fixture construction to succeed: {error:?}"),
    }
}

fn daily_calendar() -> FixedStepCalendar {
    let origin = CalendarOrigin::new(CalendarInstant::from_unix_seconds(946_684_800));
    let duration = assert_ok(TimestepDuration::from_seconds(86_400));
    FixedStepCalendar::new(origin, duration)
}

#[test]
fn duration_construction_is_checked() {
    assert_eq!(
        TimestepDuration::from_seconds(0),
        Err(TemporalError::ZeroTimestepDuration { seconds: 0 })
    );

    let duration = assert_ok(TimestepDuration::from_seconds(86_400));
    assert_eq!(duration.seconds(), 86_400);
}

#[test]
fn fixed_step_calendar_converts_in_both_directions() {
    let calendar = daily_calendar();
    assert_eq!(calendar.origin().instant().unix_seconds(), 946_684_800);
    assert_eq!(calendar.timestep_duration().seconds(), 86_400);
    assert_eq!(
        assert_ok(calendar.instant_at(TimestepIndex::new(0))).unix_seconds(),
        946_684_800
    );
    assert_eq!(
        assert_ok(calendar.instant_at(TimestepIndex::new(2))).unix_seconds(),
        946_857_600
    );
    assert_eq!(
        assert_ok(calendar.index_of(CalendarInstant::from_unix_seconds(946_684_800))).value(),
        0
    );
    assert_eq!(
        assert_ok(calendar.index_of(CalendarInstant::from_unix_seconds(946_857_600))).value(),
        2
    );
}

#[test]
fn reverse_conversion_rejects_unrepresentable_positions() {
    let origin = CalendarOrigin::new(CalendarInstant::from_unix_seconds(100));
    let duration = assert_ok(TimestepDuration::from_seconds(10));
    let calendar = FixedStepCalendar::new(origin, duration);

    assert_eq!(
        calendar.index_of(CalendarInstant::from_unix_seconds(99)),
        Err(TemporalError::InstantBeforeOrigin {
            origin_unix_seconds: 100,
            instant_unix_seconds: 99,
        })
    );
    assert_eq!(
        calendar.index_of(CalendarInstant::from_unix_seconds(111)),
        Err(TemporalError::InstantNotOnTimestep {
            origin_unix_seconds: 100,
            timestep_duration_seconds: 10,
            instant_unix_seconds: 111,
        })
    );
}

#[test]
fn calendar_arithmetic_accepts_exact_extremes_and_rejects_overflow() {
    let origin = CalendarOrigin::new(CalendarInstant::from_unix_seconds(i64::MIN));
    let duration = assert_ok(TimestepDuration::from_seconds(u64::MAX));
    let calendar = FixedStepCalendar::new(origin, duration);

    assert_eq!(
        assert_ok(calendar.instant_at(TimestepIndex::new(1))).unix_seconds(),
        i64::MAX
    );
    assert_eq!(
        assert_ok(calendar.index_of(CalendarInstant::from_unix_seconds(i64::MAX))).value(),
        1
    );
    assert_eq!(
        calendar.instant_at(TimestepIndex::new(2)),
        Err(TemporalError::CalendarArithmeticOverflow {
            origin_unix_seconds: i64::MIN,
            timestep_duration_seconds: u64::MAX,
            timestep_index: 2,
        })
    );
}

#[test]
fn calendar_arithmetic_rejects_products_wider_than_the_intermediate() {
    let origin = CalendarOrigin::new(CalendarInstant::from_unix_seconds(i64::MIN));
    let duration = assert_ok(TimestepDuration::from_seconds(u64::MAX));
    let calendar = FixedStepCalendar::new(origin, duration);
    assert_eq!(
        calendar.instant_at(TimestepIndex::new(u64::MAX)),
        Err(TemporalError::CalendarArithmeticOverflow {
            origin_unix_seconds: i64::MIN,
            timestep_duration_seconds: u64::MAX,
            timestep_index: u64::MAX,
        })
    );
}

#[test]
fn run_horizon_is_inclusive_and_checked() {
    let horizon = assert_ok(RunHorizon::new(
        TimestepIndex::new(3),
        TimestepIndex::new(5),
    ));
    assert_eq!(horizon.first().value(), 3);
    assert_eq!(horizon.last().value(), 5);
    assert!(horizon.contains(TimestepIndex::new(3)));
    assert!(horizon.contains(TimestepIndex::new(5)));
    assert!(!horizon.contains(TimestepIndex::new(2)));
    assert!(!horizon.contains(TimestepIndex::new(6)));

    let maximum = assert_ok(RunHorizon::new(
        TimestepIndex::new(u64::MAX),
        TimestepIndex::new(u64::MAX),
    ));
    assert_eq!(maximum.first().value(), u64::MAX);
    assert_eq!(maximum.last().value(), u64::MAX);
    assert!(maximum.contains(TimestepIndex::new(u64::MAX)));

    assert_eq!(
        RunHorizon::new(TimestepIndex::new(5), TimestepIndex::new(4)),
        Err(TemporalError::ReversedRunHorizon {
            first_timestep: 5,
            last_timestep: 4,
        })
    );
}
