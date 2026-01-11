use rurl::output::ProgressReporter;

#[test]
fn progress_reporter_handles_zero_total_and_finish() {
    let mut reporter = ProgressReporter::new(true, Some(0));
    reporter.update(0);
    reporter.finish(0);
    assert!(reporter.rendered(), "should render at least once");
}
