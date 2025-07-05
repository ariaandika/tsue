use std::time::{SystemTime, UNIX_EPOCH};

/// Create [httpdate][rfc] for current time.
///
/// [rfc]: <https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.7>
#[inline]
pub fn httpdate_now() -> [u8; 29] {
    httpdate(SystemTime::now())
}

/// Create [httpdate][rfc] with given time.
///
/// [rfc]: <https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.7>
pub fn httpdate(v: SystemTime) -> [u8; 29] {
    let dur = v.duration_since(UNIX_EPOCH).unwrap();

    let secs_since_epoch = dur.as_secs();
    if secs_since_epoch >= 253402300800 {
        // year 9999
        panic!("date must be before year 9999");
    }

    /* 2000-03-01 (mod 400 year, immediately after feb29 */

    const LEAPOCH: i64 = 11017;
    const DAYS_PER_400Y: i64 = 365 * 400 + 97;
    const DAYS_PER_100Y: i64 = 365 * 100 + 24;
    const DAYS_PER_4Y: i64 = 365 * 4 + 1;

    let days = (secs_since_epoch / 86400) as i64 - LEAPOCH;
    let secs_of_day = secs_since_epoch % 86400;

    let mut qc_cycles = days / DAYS_PER_400Y;
    let mut remdays = days % DAYS_PER_400Y;

    if remdays < 0 {
        remdays += DAYS_PER_400Y;

        qc_cycles -= 1;
    }

    let mut c_cycles = remdays / DAYS_PER_100Y;
    if c_cycles == 4 {
        c_cycles -= 1;
    }
    remdays -= c_cycles * DAYS_PER_100Y;

    let mut q_cycles = remdays / DAYS_PER_4Y;
    if q_cycles == 25 {
        q_cycles -= 1;
    }
    remdays -= q_cycles * DAYS_PER_4Y;

    let mut remyears = remdays / 365;
    if remyears == 4 {
        remyears -= 1;
    }
    remdays -= remyears * 365;

    let mut year = 2000 + remyears + 4 * q_cycles + 100 * c_cycles + 400 * qc_cycles;

    let months = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
    let mut mon = 0;
    for mon_len in months.iter() {
        mon += 1;
        if remdays < *mon_len {
            break;
        }
        remdays -= *mon_len;
    }
    let mday = remdays + 1;
    let mon = if mon + 2 > 12 {
        year += 1;
        mon - 10
    } else {
        mon + 2
    };

    // ===== Write =====
    // https://www.rfc-editor.org/rfc/rfc9110#section-5.6.7

    let mut buf: [u8; 29] = *b"ddd, 00 mmm 1970 00:00:00 GMT";

    // ===== day-name =====

    let mut wday = (3 + days) % 7;
    if wday <= 0 {
        wday += 7
    };
    buf[..3].copy_from_slice(match wday {
        1 => b"Mon",
        2 => b"Tue",
        3 => b"Wed",
        4 => b"Thu",
        5 => b"Fri",
        6 => b"Sat",
        7 => b"Sun",
        _ => unreachable!(),
    });

    // ===== day =====

    let day = mday as u8;
    buf[5] = b'0' + (day / 10);
    buf[6] = b'0' + (day % 10);

    // ===== month =====

    buf[8..11].copy_from_slice(match mon {
        1 => b"Jan",
        2 => b"Feb",
        3 => b"Mar",
        4 => b"Apr",
        5 => b"May",
        6 => b"Jun",
        7 => b"Jul",
        8 => b"Aug",
        9 => b"Sep",
        10 => b"Oct",
        11 => b"Nov",
        12 => b"Dec",
        _ => unreachable!(),
    });

    // ===== year =====

    buf[12] = b'0' + (year / 1000) as u8;
    buf[13] = b'0' + (year / 100 % 10) as u8;
    buf[14] = b'0' + (year / 10 % 10) as u8;
    buf[15] = b'0' + (year % 10) as u8;

    // ===== hour =====

    let hour = (secs_of_day / 3600) as u8;
    buf[17] = b'0' + (hour / 10);
    buf[18] = b'0' + (hour % 10);

    // ===== minute =====

    let min = ((secs_of_day % 3600) / 60) as u8;
    buf[20] = b'0' + (min / 10);
    buf[21] = b'0' + (min % 10);

    // ===== second =====

    let sec = (secs_of_day % 60) as u8;
    buf[23] = b'0' + (sec / 10);
    buf[24] = b'0' + (sec % 10);


    buf
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};
    use super::httpdate;

    #[test]
    fn test_httpdate() {
        let d = UNIX_EPOCH;
        assert_eq!(str::from_utf8(&httpdate(d)), Ok("Thu, 01 Jan 1970 00:00:00 GMT"));
        let d = UNIX_EPOCH + Duration::from_secs(1475419451);
        assert_eq!(str::from_utf8(&httpdate(d)), Ok("Sun, 02 Oct 2016 14:44:11 GMT"));
    }
}

