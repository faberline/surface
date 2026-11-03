use super::FiberId;

#[test]
fn fiber_id_round_trips_its_raw_value() {
    let id = FiberId::new(7);
    assert_eq!(id.get(), 7);
    assert_eq!(id, FiberId::new(7));
    assert_ne!(id, FiberId::new(8));
    assert_eq!(FiberId::default().get(), 0);
}

#[test]
fn fiber_id_debug_output_is_the_tuple_form() {
    assert_eq!(format!("{:?}", FiberId::new(42)), "FiberId(42)");
}
