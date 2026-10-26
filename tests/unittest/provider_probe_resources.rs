//! Portable production resource-admission tests with owned byte observations.
use super::probe::{journal::Failure, native::ResourceBudget};
const MIB: u64 = 1_048_576;
const INITIAL: (u64, u64) = (4_430_233_600, 3_591_372_800);

#[test]
fn deferred_npm_resource_admission_accepts_exact_reviewed_space() {
    let budget = ResourceBudget::new(0, 0, 0);
    assert_eq!(
        budget.admit(Ok(0), Ok(INITIAL.0), Ok(INITIAL.1)),
        Ok(()),
        "fixed deferred-npm delivery and helper must fit the reviewed exact boundary"
    );
    assert_eq!(budget.requirements().unwrap(), INITIAL);
    assert_eq!(
        budget.admit(Ok(0), Ok(INITIAL.0 - 1), Ok(INITIAL.1)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(INITIAL.0), Ok(INITIAL.1 - 1)),
        Err(Failure::Resource)
    );
}

#[test]
fn retained_helper_consumption_is_counted_once_without_retiring_delivery_reserves() {
    // Retained C16MiB, launcher512KiB, runtime4MiB: the reviewed worked totals.
    let budget = ResourceBudget::new(16 * MIB, MIB / 2, 4 * MIB);
    assert_eq!(
        budget.requirements().unwrap(),
        (4_413_456_384, 3_586_654_208)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(4_413_456_384), Ok(3_586_654_208)),
        Ok(())
    );
    assert_eq!(budget.high_water().unwrap(), 4 * MIB);
    assert_eq!(
        budget.admit(Ok(0), Ok(4_413_456_383), Ok(3_586_654_208)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(4_413_456_384), Ok(3_586_654_207)),
        Err(Failure::Resource)
    );
}

#[test]
fn unavailable_or_shrinking_observations_fail_closed_and_keep_failed_write_peak() {
    let budget = ResourceBudget::new(0, 0, 0);
    assert_eq!(
        budget.admit(Err(Failure::Resource), Ok(u64::MAX), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(4 * MIB), Err(Failure::Resource), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(budget.high_water().unwrap(), 4 * MIB);
    assert_eq!(
        budget.admit(Ok(0), Ok(u64::MAX), Err(Failure::Resource)),
        Err(Failure::Resource)
    );
    // A cancelled/failed write shrank on disk; it does not grant a new runtime budget.
    assert_eq!(budget.high_water().unwrap(), 4 * MIB);
    assert_eq!(
        budget.requirements().unwrap(),
        (4_430_233_600, 3_587_178_496)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(4_430_233_599), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(u64::MAX), Ok(3_587_178_495)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(4_430_233_600), Ok(3_587_178_496)),
        Ok(())
    );
    assert_eq!(
        budget.admit(Ok(24 * MIB), Ok(u64::MAX), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(
        budget.admit(Ok(0), Ok(u64::MAX), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(budget.high_water().unwrap(), 24 * MIB);
}

#[test]
fn fixed_caps_stops_and_extreme_high_water_marks_cannot_wrap_or_reset() {
    let within = ResourceBudget::new(112 * MIB, MIB, 24 * MIB - 1);
    assert_eq!(within.admit(Ok(0), Ok(u64::MAX), Ok(u64::MAX)), Ok(()));
    for (c, launcher, runtime) in [
        (112 * MIB + 1, 0, 0),
        (128 * MIB, 0, 0),
        (0, MIB + 1, 0),
        (0, 0, 24 * MIB),
        (0, 0, 32 * MIB),
        (u64::MAX, 0, 0),
        (0, u64::MAX, 0),
        (0, 0, u64::MAX),
    ] {
        let budget = ResourceBudget::new(c, launcher, runtime);
        assert_eq!(budget.requirements(), Err(Failure::Resource));
        assert_eq!(
            budget.admit(Ok(0), Ok(u64::MAX), Ok(u64::MAX)),
            Err(Failure::Resource)
        );
    }
    let overflow = ResourceBudget::new(0, 0, 0);
    assert_eq!(
        overflow.admit(Ok(u64::MAX), Ok(u64::MAX), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
    assert_eq!(
        overflow.admit(Ok(0), Ok(u64::MAX), Ok(u64::MAX)),
        Err(Failure::Resource)
    );
}
