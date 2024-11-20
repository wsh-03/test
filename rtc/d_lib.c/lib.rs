
use kernel::bindings::*;

static RTC_DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

static RTC_YDAYS: [[u16; 13]; 2] = [
    [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365],
    [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366],
];

#[no_mangle]
pub extern "C" fn rtc_month_days(month: u32, year: u32) -> i32 {
    unsafe {
        RTC_DAYS_IN_MONTH.get_unchecked(month as usize) as i32
            + if is_leap_year(year) && month == 1 { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn rtc_year_days(day: u32, month: u32, year: u32) -> i32 {
    unsafe { RTC_YDAYS[is_leap_year(year) as usize][month as usize] as i32 + day as i32 - 1 }
}

#[no_mangle]
pub extern "C" fn rtc_time64_to_tm(time: i64, tm: *mut rtc_time) {
    let mut secs: u32;
    let mut days: i32;

    let u64tmp: u64;
    let mut u32tmp: u32;
    let mut udays: u32;
    let mut century: u32;
    let mut day_of_century: u32;
    let mut year_of_century: u32;
    let mut year: u32;
    let mut day_of_year: u32;
    let mut month: u32;
    let mut day: u32;
    let mut is_Jan_or_Feb: bool;
    let mut is_leap_year: bool;

    days = unsafe { div_s64_rem(time, 86400, &mut secs) };

    unsafe { (*tm).tm_wday = (days + 4) % 7 };

    udays = (days as u32) + 719468;

    u32tmp = 4 * udays + 3;
    century = u32tmp / 146097;
    day_of_century = u32tmp % 146097 / 4;

    u32tmp = 4 * day_of_century + 3;
    u64tmp = 2939745u64 * u32tmp as u64;
    year_of_century = (u64tmp >> 32) as u32;
    day_of_year = ((u64tmp & 0xFFFFFFFF) as u32 / 2939745) / 4;

    year = 100 * century + year_of_century;
    is_leap_year = year_of_century != 0 && year_of_century % 4 == 0 || century % 4 == 0;

    u32tmp = 2141 * day_of_year + 132377;
    month = u32tmp >> 16;
    day = (u32tmp & 0xFFFF) / 2141;

    is_Jan_or_Feb = day_of_year >= 306;

    year = year + is_Jan_or_Feb as u32;
    month = if is_Jan_or_Feb { month - 12 } else { month };
    day = day + 1;

    day_of_year = if is_Jan_or_Feb {
        day_of_year - 306
    } else {
        day_of_year + 31 + 28 + is_leap_year as u32
    };

    let tm_ref = unsafe { &mut *tm };

    tm_ref.tm_year = (year - 1900) as i32;
    tm_ref.tm_mon = month as i32;
    tm_ref.tm_mday = day as i32;
    tm_ref.tm_yday = (day_of_year + 1) as i32;

    tm_ref.tm_hour = secs / 3600;
    secs -= tm_ref.tm_hour * 3600;
    tm_ref.tm_min = secs / 60;
    tm_ref.tm_sec = secs - tm_ref.tm_min * 60;

    tm_ref.tm_isdst = 0;
}

#[no_mangle]
pub extern "C" fn rtc_valid_tm(tm: *mut rtc_time) -> i32 {
    let tm_ref = unsafe { &*tm };
    if tm_ref.tm_year < 70
        || tm_ref.tm_year > (i32::MAX - 1900)
        || (tm_ref.tm_mon as u32) >= 12
        || tm_ref.tm_mday < 1
        || tm_ref.tm_mday > rtc_month_days(tm_ref.tm_mon as u32, (tm_ref.tm_year as u32 + 1900))
        || (tm_ref.tm_hour as u32) >= 24
        || (tm_ref.tm_min as u32) >= 60
        || (tm_ref.tm_sec as u32) >= 60
    {
        -EINVAL
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn rtc_tm_to_time64(tm: *mut rtc_time) -> time64_t {
    let tm_ref = unsafe { &mut *tm };
    unsafe {
        mktime64(
            tm_ref.tm_year as u32 + 1900,
            tm_ref.tm_mon as u32 + 1,
            tm_ref.tm_mday as u32,
            tm_ref.tm_hour as u32,
            tm_ref.tm_min as u32,
            tm_ref.tm_sec as u32,
        )
    }
}

#[no_mangle]
pub extern "C" fn rtc_tm_to_ktime(mut tm: rtc_time) -> ktime_t {
    unsafe { ktime_set(rtc_tm_to_time64(&mut tm), 0) }
}

#[no_mangle]
pub extern "C" fn rtc_ktime_to_tm(kt: ktime_t) -> rtc_time {
    let mut ts: timespec64;
    let mut ret = core::mem::MaybeUninit::uninit();
    ts = unsafe { ktime_to_timespec64(kt) };
    if ts.tv_nsec != 0 {
        ts.tv_sec += 1;
    }
    unsafe { rtc_time64_to_tm(ts.tv_sec, ret.as_mut_ptr()) };
    unsafe { ret.assume_init() }
}
