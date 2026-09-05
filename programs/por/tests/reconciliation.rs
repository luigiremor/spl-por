mod common;

// mint decimals 6, reserve_decimals 2 => factor 10^4

#[test]
fn overcollateralized_is_ok() {
    // R_norm = 200 * 10^4 = 2_000_000 > supply 1_000_000
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    common::send_submit(&mut env, &oracle, common::default_args(0)).unwrap();
    assert_eq!(common::get_record(&env, 0).status, por::state::STATUS_OK);
}

#[test]
fn exact_coverage_is_ok() {
    // R_norm = 100 * 10^4 = 1_000_000 == supply
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let mut args = common::default_args(0);
    args.total_reservas = 100;
    common::send_submit(&mut env, &oracle, args).unwrap();
    assert_eq!(common::get_record(&env, 0).status, por::state::STATUS_OK);
}

#[test]
fn undercollateralized_is_divergente_and_recorded() {
    // R_norm = 50 * 10^4 = 500_000 < supply => DIVERGENTE, but the attestation
    // IS ACCEPTED and recorded (a divergence is never silently dropped)
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let mut args = common::default_args(0);
    args.total_reservas = 50;
    common::send_submit(&mut env, &oracle, args).unwrap();
    let record = common::get_record(&env, 0);
    assert_eq!(record.status, por::state::STATUS_DIVERGENTE);
    assert_eq!(record.supply_at_ingestion, 1_000_000);
    assert_eq!(common::get_config(&env).next_sequence, 1);
}

#[test]
fn normalization_overflow_aborts() {
    // decimals 9 => factor 10^7; total_reservas 10^19 * 10^7 overflows u64
    let mut env = common::setup(1_000_000, 9);
    let oracle = env.oracle.insecure_clone();
    let mut args = common::default_args(0);
    args.decimals = 9;
    args.total_reservas = 10_000_000_000_000_000_000; // 10^19
    let res = common::send_submit(&mut env, &oracle, args);
    common::assert_por_error(res, 7); // Overflow = 6007
}
