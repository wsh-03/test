
use kernel::bindings::*;
use kernel::prelude::*;

unsafe fn advance_date(year: &mut i32, month: &mut i32, mday: &mut i32, yday: &mut i32) {
    if *mday != rtc_month_days(*month - 1, *year) {
        *mday += 1;
        *yday += 1;
        return;
    }

    *mday = 1;
    if *month != 12 {
        *month += 1;
        *yday += 1;
        return;
    }

    *month = 1;
    *yday = 1;
    *year += 1;
}

unsafe fn rtc_time64_to_tm_test_date_range(test: *mut kunit, years: i32) {
    let total_secs: time64_t = (years as time64_t / 400) * 146097 * 86400;

    let mut year = 1970;
    let mut month = 1;
    let mut mday = 1;
    let mut yday = 1;

    let mut result = core::mem::zeroed::<rtc_time>();
    let mut secs: time64_t;
    let mut days: s64;

    secs = 0;
    while secs <= total_secs {
        rtc_time64_to_tm(secs, &mut result);
        days = div_s64(secs, 86400);

        let fail_msg = format!(
            "{}/{}: ({:02}), {}",
            year, month, mday, yday, days
        );

        kunit_assert_eq_msg(test, year - 1900, result.tm_year as i32, &fail_msg);
        kunit_assert_eq_msg(test, month - 1, result.tm_mon as i32, &fail_msg);
        kunit_assert_eq_msg(test, mday, result.tm_mday as i32, &fail_msg);
        kunit_assert_eq_msg(test, yday, result.tm_yday as i32, &fail_msg);

        advance_date(&mut year, &mut month, &mut mday, &mut yday);
        secs += 86400;
    }
}

unsafe fn rtc_time64_to_tm_test_date_range_160000(test: *mut kunit) {
    rtc_time64_to_tm_test_date_range(test, 160000);
}

unsafe fn rtc_time64_to_tm_test_date_range_1000(test: *mut kunit) {
    rtc_time64_to_tm_test_date_range(test, 1000);
}

static RTC_LIB_TEST_CASES: kunit_case = kunit_case {
    run_case: Some(rtc_time64_to_tm_test_date_range_1000),
    name: b"rtc_time64_to_tm_test_date_range_1000\0" as *const _ as *const i8,
    ..Default::default()
};

static RTC_LIB_TEST_CASES_SLOW: kunit_case = kunit_case {
    run_case: Some(rtc_time64_to_tm_test_date_range_160000),
    name: b"rtc_time64_to_tm_test_date_range_160000\0" as *const _ as *const i8,
    ..Default::default()
};

static RTC_LIB_TEST_SUITE: kunit_suite = kunit_suite {
    name: b"rtc_lib_test_cases\0" as *const _ as *const i8,
    test_cases: [RTC_LIB_TEST_CASES, RTC_LIB_TEST_CASES_SLOW, kunit_case::default()],
    ..Default::default()
};

module_init!();
module_desc!("KUnit test for RTC lib functions");
module_license!("GPL");
