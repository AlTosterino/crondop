use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::{AppConfig, ScheduleMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleKind {
    Interval,
    FixedTimes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextReminder {
    pub at: DateTime<Local>,
    pub kind: ScheduleKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderAction {
    Done,
    Snooze,
    Skip,
    PauseToday,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    ClearActive,
    SnoozedUntil(DateTime<Local>),
    PausedUntil(NaiveDate),
}

pub fn next_reminder_after(
    config: &AppConfig,
    after: DateTime<Local>,
    snoozed_until: Option<DateTime<Local>>,
) -> Result<NextReminder> {
    next_reminder_after_with_anchor(config, after, snoozed_until, None)
}

pub fn next_reminder_after_with_anchor(
    config: &AppConfig,
    after: DateTime<Local>,
    snoozed_until: Option<DateTime<Local>>,
    anchor: Option<DateTime<Local>>,
) -> Result<NextReminder> {
    if let Some(snoozed_until) = snoozed_until {
        if snoozed_until > after {
            return Ok(NextReminder {
                at: snoozed_until,
                kind: ScheduleKind::Interval,
            });
        }
    }

    match config.schedule.mode {
        ScheduleMode::Interval => next_interval_reminder(config, after, anchor),
        ScheduleMode::FixedTimes => next_fixed_time_reminder(config, after),
    }
}

pub fn previous_reminder_before(
    config: &AppConfig,
    before: DateTime<Local>,
) -> Result<NextReminder> {
    match config.schedule.mode {
        ScheduleMode::Interval => previous_interval_reminder(config, before),
        ScheduleMode::FixedTimes => previous_fixed_time_reminder(config, before),
    }
}

pub fn outcome_for_action(
    config: &AppConfig,
    now: DateTime<Local>,
    action: ReminderAction,
) -> ActionOutcome {
    match action {
        ReminderAction::Done | ReminderAction::Skip => ActionOutcome::ClearActive,
        ReminderAction::Snooze => {
            ActionOutcome::SnoozedUntil(now + Duration::minutes(config.popup.snooze_minutes as i64))
        }
        ReminderAction::PauseToday => ActionOutcome::PausedUntil(now.date_naive()),
    }
}

fn next_interval_reminder(
    config: &AppConfig,
    after: DateTime<Local>,
    anchor: Option<DateTime<Local>>,
) -> Result<NextReminder> {
    let start = parse_clock(&config.schedule.active_from)?;
    let end = parse_clock(&config.schedule.active_to)?;

    let interval = Duration::minutes(config.schedule.every_minutes as i64);

    if let Some(anchor) = anchor {
        let mut candidate = anchor + interval;
        while candidate <= after {
            candidate += interval;
        }

        for _ in 0..14 {
            let day = candidate.date_naive();
            if !is_allowed_day(day, config.schedule.weekdays_only) {
                candidate = next_allowed_day_start(
                    config,
                    day.succ_opt().context("failed to advance day")?,
                )?;
                continue;
            }

            let day_start = combine_local(day, start)?;
            let day_end = combine_local(day, end)?;
            if day_end <= day_start {
                candidate = next_allowed_day_start(
                    config,
                    day.succ_opt().context("failed to advance day")?,
                )?;
                continue;
            }

            if candidate < day_start {
                candidate = day_start;
            }

            if candidate <= day_end {
                return Ok(NextReminder {
                    at: candidate,
                    kind: ScheduleKind::Interval,
                });
            }

            candidate =
                next_allowed_day_start(config, day.succ_opt().context("failed to advance day")?)?;
        }

        anyhow::bail!("failed to find anchored interval reminder in search window");
    }

    let mut day = after.date_naive();
    for _ in 0..14 {
        if !is_allowed_day(day, config.schedule.weekdays_only) {
            day = day.succ_opt().context("failed to advance day")?;
            continue;
        }

        let day_start = combine_local(day, start)?;
        let day_end = combine_local(day, end)?;

        if day_end <= day_start {
            day = day.succ_opt().context("failed to advance day")?;
            continue;
        }

        let candidate = if after < day_start {
            day_start
        } else {
            let elapsed = after - day_start;
            let minutes = elapsed.num_minutes().max(0);
            let step = config.schedule.every_minutes.max(1) as i64;
            let next_step = (minutes / step) + 1;
            day_start + interval * (next_step as i32)
        };

        if candidate <= day_end {
            return Ok(NextReminder {
                at: candidate,
                kind: ScheduleKind::Interval,
            });
        }

        day = day.succ_opt().context("failed to advance day")?;
    }

    anyhow::bail!("failed to find next interval reminder in search window")
}

fn next_fixed_time_reminder(config: &AppConfig, after: DateTime<Local>) -> Result<NextReminder> {
    let fixed_times = if config.schedule.fixed_times.is_empty() {
        vec![config.schedule.active_from.clone()]
    } else {
        config.schedule.fixed_times.clone()
    };

    let parsed_times = fixed_times
        .iter()
        .map(|value| parse_clock(value))
        .collect::<Result<Vec<_>>>()?;

    let mut day = after.date_naive();

    for _ in 0..14 {
        if !is_allowed_day(day, config.schedule.weekdays_only) {
            day = day.succ_opt().context("failed to advance day")?;
            continue;
        }

        for time in &parsed_times {
            let candidate = combine_local(day, *time)?;
            if candidate > after {
                return Ok(NextReminder {
                    at: candidate,
                    kind: ScheduleKind::FixedTimes,
                });
            }
        }

        day = day.succ_opt().context("failed to advance day")?;
    }

    anyhow::bail!("failed to find next fixed-time reminder in search window")
}

fn previous_interval_reminder(config: &AppConfig, before: DateTime<Local>) -> Result<NextReminder> {
    let start = parse_clock(&config.schedule.active_from)?;
    let end = parse_clock(&config.schedule.active_to)?;
    let mut day = before.date_naive();
    let interval = Duration::minutes(config.schedule.every_minutes.max(1) as i64);

    for _ in 0..14 {
        if !is_allowed_day(day, config.schedule.weekdays_only) {
            day = day.pred_opt().context("failed to move to previous day")?;
            continue;
        }

        let day_start = combine_local(day, start)?;
        let day_end = combine_local(day, end)?;

        if day_end <= day_start {
            day = day.pred_opt().context("failed to move to previous day")?;
            continue;
        }

        if before > day_start {
            let elapsed = (before - day_start).num_minutes().max(0);
            let step = config.schedule.every_minutes.max(1) as i64;
            let previous_step = ((elapsed - 1).max(0)) / step;
            let candidate = day_start + interval * (previous_step as i32);

            if candidate >= day_start && candidate <= day_end && candidate < before {
                return Ok(NextReminder {
                    at: candidate,
                    kind: ScheduleKind::Interval,
                });
            }
        }

        day = day.pred_opt().context("failed to move to previous day")?;
    }

    anyhow::bail!("failed to find previous interval reminder in search window")
}

fn previous_fixed_time_reminder(
    config: &AppConfig,
    before: DateTime<Local>,
) -> Result<NextReminder> {
    let fixed_times = if config.schedule.fixed_times.is_empty() {
        vec![config.schedule.active_from.clone()]
    } else {
        config.schedule.fixed_times.clone()
    };

    let mut parsed_times = fixed_times
        .iter()
        .map(|value| parse_clock(value))
        .collect::<Result<Vec<_>>>()?;
    parsed_times.sort();

    let mut day = before.date_naive();

    for _ in 0..14 {
        if !is_allowed_day(day, config.schedule.weekdays_only) {
            day = day.pred_opt().context("failed to move to previous day")?;
            continue;
        }

        for time in parsed_times.iter().rev() {
            let candidate = combine_local(day, *time)?;
            if candidate < before {
                return Ok(NextReminder {
                    at: candidate,
                    kind: ScheduleKind::FixedTimes,
                });
            }
        }

        day = day.pred_opt().context("failed to move to previous day")?;
    }

    anyhow::bail!("failed to find previous fixed-time reminder in search window")
}

fn parse_clock(value: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .with_context(|| format!("invalid time value `{value}`; expected HH:MM"))
}

fn combine_local(date: NaiveDate, time: NaiveTime) -> Result<DateTime<Local>> {
    let naive = date.and_time(time);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Ok(value),
        LocalResult::Ambiguous(first, second) => Ok(first.min(second)),
        LocalResult::None => anyhow::bail!("local time does not exist for {}", naive),
    }
}

fn is_allowed_day(date: NaiveDate, weekdays_only: bool) -> bool {
    if !weekdays_only {
        return true;
    }

    !matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
}

fn next_allowed_day_start(config: &AppConfig, mut day: NaiveDate) -> Result<DateTime<Local>> {
    let start = parse_clock(&config.schedule.active_from)?;
    for _ in 0..14 {
        if is_allowed_day(day, config.schedule.weekdays_only) {
            return combine_local(day, start);
        }
        day = day.succ_opt().context("failed to advance day")?;
    }

    anyhow::bail!("failed to find next allowed day start in search window")
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use crate::{AppConfig, ScheduleMode};

    use super::{next_reminder_after, next_reminder_after_with_anchor, previous_reminder_before};

    fn local_datetime(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> chrono::DateTime<Local> {
        Local
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .single()
            .expect("valid local datetime")
    }

    #[test]
    fn interval_schedule_advances_on_grid() {
        let config = AppConfig::default();
        let now = local_datetime(2026, 3, 30, 8, 15);
        let next = next_reminder_after(&config, now, None).expect("next reminder");
        assert_eq!(next.at, local_datetime(2026, 3, 30, 9, 0));
    }

    #[test]
    fn interval_schedule_rolls_to_next_day_after_window() {
        let config = AppConfig::default();
        let now = local_datetime(2026, 3, 30, 22, 30);
        let next = next_reminder_after(&config, now, None).expect("next reminder");
        assert_eq!(next.at, local_datetime(2026, 3, 31, 8, 0));
    }

    #[test]
    fn fixed_times_selects_next_matching_entry() {
        let mut config = AppConfig::default();
        config.schedule.mode = ScheduleMode::FixedTimes;
        config.schedule.fixed_times = vec!["09:00".into(), "13:00".into(), "18:00".into()];

        let now = local_datetime(2026, 3, 30, 12, 0);
        let next = next_reminder_after(&config, now, None).expect("next reminder");
        assert_eq!(next.at, local_datetime(2026, 3, 30, 13, 0));
    }

    #[test]
    fn anchored_interval_uses_cycle_start_instead_of_clock_grid() {
        let mut config = AppConfig::default();
        config.schedule.every_minutes = 1;
        let anchor = local_datetime(2026, 3, 30, 21, 5);
        let after = local_datetime(2026, 3, 30, 21, 5);
        let next =
            next_reminder_after_with_anchor(&config, after, None, Some(anchor)).expect("next");
        assert_eq!(next.at, local_datetime(2026, 3, 30, 21, 6));
    }

    #[test]
    fn interval_schedule_finds_previous_slot() {
        let config = AppConfig::default();
        let now = local_datetime(2026, 3, 30, 10, 15);
        let previous = previous_reminder_before(&config, now).expect("previous reminder");
        assert_eq!(previous.at, local_datetime(2026, 3, 30, 10, 0));
    }
}
