//! fixed_step_time : FixedStepCalendar × TimestepIndex ⇄ CalendarInstant; RunHorizon ⊆ TimestepIndex   (pure, checked)

use std::num::NonZeroU64;

use thiserror::Error;

/// An error constructing or operating on fixed-step temporal domain types.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TemporalError {
    /// Fires when a timestep duration of zero seconds is requested.
    #[error("timestep duration must be positive, but received {seconds} seconds")]
    ZeroTimestepDuration { seconds: u64 },
    /// Fires when a calendar instant cannot be represented as signed Unix seconds.
    #[error(
        "calendar arithmetic overflow for origin {origin_unix_seconds}, duration {timestep_duration_seconds} seconds, and index {timestep_index}"
    )]
    CalendarArithmeticOverflow {
        origin_unix_seconds: i64,
        timestep_duration_seconds: u64,
        timestep_index: u64,
    },
    /// Fires when an instant precedes the calendar origin.
    #[error(
        "instant {instant_unix_seconds} precedes calendar origin {origin_unix_seconds} Unix seconds"
    )]
    InstantBeforeOrigin {
        origin_unix_seconds: i64,
        instant_unix_seconds: i64,
    },
    /// Fires when an instant does not align with a fixed timestep.
    #[error(
        "instant {instant_unix_seconds} is not on the calendar with origin {origin_unix_seconds} and duration {timestep_duration_seconds} seconds"
    )]
    InstantNotOnTimestep {
        origin_unix_seconds: i64,
        timestep_duration_seconds: u64,
        instant_unix_seconds: i64,
    },
    /// Fires when the last timestep precedes the first timestep.
    #[error(
        "run horizon is reversed: first timestep {first_timestep}, last timestep {last_timestep}"
    )]
    ReversedRunHorizon {
        first_timestep: u64,
        last_timestep: u64,
    },
}

/// An exact positive whole number of seconds between timesteps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimestepDuration(NonZeroU64);

impl TimestepDuration {
    /// Constructs a positive timestep duration from whole seconds.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalError::ZeroTimestepDuration`] when `seconds` is zero.
    pub fn from_seconds(seconds: u64) -> Result<Self, TemporalError> {
        NonZeroU64::new(seconds)
            .map(Self)
            .ok_or(TemporalError::ZeroTimestepDuration { seconds })
    }

    /// Returns the duration in whole seconds.
    #[must_use]
    pub const fn seconds(self) -> u64 {
        self.0.get()
    }
}

/// A zero-based timestep ordinal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimestepIndex(u64);

impl TimestepIndex {
    /// Constructs a timestep index.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the zero-based ordinal.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A calendar coordinate expressed as signed Unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarInstant(i64);

impl CalendarInstant {
    /// Constructs a calendar instant from signed Unix seconds.
    #[must_use]
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Returns the signed Unix-second coordinate.
    #[must_use]
    pub const fn unix_seconds(self) -> i64 {
        self.0
    }
}

/// The typed origin of a fixed-step calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarOrigin(CalendarInstant);

impl CalendarOrigin {
    /// Constructs a calendar origin.
    #[must_use]
    pub const fn new(instant: CalendarInstant) -> Self {
        Self(instant)
    }

    /// Returns the origin instant.
    #[must_use]
    pub const fn instant(self) -> CalendarInstant {
        self.0
    }
}

/// A calendar defined by one origin and one fixed positive duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedStepCalendar {
    origin: CalendarOrigin,
    timestep_duration: TimestepDuration,
}

impl FixedStepCalendar {
    /// Constructs a fixed-step calendar.
    #[must_use]
    pub const fn new(origin: CalendarOrigin, timestep_duration: TimestepDuration) -> Self {
        Self {
            origin,
            timestep_duration,
        }
    }

    /// Returns the calendar origin.
    #[must_use]
    pub const fn origin(self) -> CalendarOrigin {
        self.origin
    }

    /// Returns the fixed timestep duration.
    #[must_use]
    pub const fn timestep_duration(self) -> TimestepDuration {
        self.timestep_duration
    }

    /// Resolves a timestep index to its calendar instant.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalError::CalendarArithmeticOverflow`] when the exact instant is outside
    /// the signed Unix-second range.
    pub fn instant_at(self, index: TimestepIndex) -> Result<CalendarInstant, TemporalError> {
        let origin_unix_seconds = self.origin.instant().unix_seconds();
        let timestep_duration_seconds = self.timestep_duration.seconds();
        let timestep_index = index.value();
        let resolved = i128::from(timestep_duration_seconds)
            .checked_mul(i128::from(timestep_index))
            .and_then(|scaled| scaled.checked_add(i128::from(origin_unix_seconds)))
            .and_then(|exact| i64::try_from(exact).ok())
            .map(CalendarInstant::from_unix_seconds)
            .ok_or(TemporalError::CalendarArithmeticOverflow {
                origin_unix_seconds,
                timestep_duration_seconds,
                timestep_index,
            })?;

        Ok(resolved)
    }

    /// Resolves an aligned calendar instant to its timestep index.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalError::InstantBeforeOrigin`] when the instant precedes the origin, or
    /// [`TemporalError::InstantNotOnTimestep`] when the instant is not aligned to the fixed step.
    pub fn index_of(self, instant: CalendarInstant) -> Result<TimestepIndex, TemporalError> {
        let origin_unix_seconds = self.origin.instant().unix_seconds();
        let instant_unix_seconds = instant.unix_seconds();
        if instant_unix_seconds < origin_unix_seconds {
            return Err(TemporalError::InstantBeforeOrigin {
                origin_unix_seconds,
                instant_unix_seconds,
            });
        }

        let timestep_duration_seconds = self.timestep_duration.seconds();
        let offset = i128::from(instant_unix_seconds) - i128::from(origin_unix_seconds);
        let duration = i128::from(timestep_duration_seconds);
        if offset % duration != 0 {
            return Err(TemporalError::InstantNotOnTimestep {
                origin_unix_seconds,
                timestep_duration_seconds,
                instant_unix_seconds,
            });
        }

        Ok(TimestepIndex::new((offset / duration) as u64))
    }
}

/// An inclusive bounded interval of timestep indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunHorizon {
    first: TimestepIndex,
    last: TimestepIndex,
}

impl RunHorizon {
    /// Constructs an inclusive run horizon.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalError::ReversedRunHorizon`] when `last` precedes `first`.
    pub fn new(first: TimestepIndex, last: TimestepIndex) -> Result<Self, TemporalError> {
        if last < first {
            return Err(TemporalError::ReversedRunHorizon {
                first_timestep: first.value(),
                last_timestep: last.value(),
            });
        }

        Ok(Self { first, last })
    }

    /// Returns the first included timestep.
    #[must_use]
    pub const fn first(self) -> TimestepIndex {
        self.first
    }

    /// Returns the last included timestep.
    #[must_use]
    pub const fn last(self) -> TimestepIndex {
        self.last
    }

    /// Reports whether the timestep lies within the inclusive horizon.
    #[must_use]
    pub fn contains(self, index: TimestepIndex) -> bool {
        self.first <= index && index <= self.last
    }
}
