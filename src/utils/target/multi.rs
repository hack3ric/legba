use crate::session::Error;

use cidr_utils::cidr::IpCidr;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref IPV4_RANGE_PARSER: Regex =
        Regex::new(r"^(\d+)\.(\d+)\.(\d+)\.(\d+)-(\d+):?(\d+)?$").unwrap();
}

pub(crate) fn parse_multiple_targets(expression: &str) -> Result<Vec<String>, Error> {
    if expression.contains(',') {
        // comma separated targets
        return Ok(expression
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect());
    } else if let Some(caps) = IPV4_RANGE_PARSER.captures(expression) {
        // ipv4 range like 192.168.1.1-10 or 192.168.1.1-10:port
        let a: u8 = caps.get(1).unwrap().as_str().parse().unwrap();
        let b: u8 = caps.get(2).unwrap().as_str().parse().unwrap();
        let c: u8 = caps.get(3).unwrap().as_str().parse().unwrap();
        let start: u8 = caps.get(4).unwrap().as_str().parse().unwrap();
        let stop: u8 = caps.get(5).unwrap().as_str().parse().unwrap();

        if stop < start {
            return Err(format!(
                "invalid ip range {}, {} is greater than {}",
                expression, start, stop
            ));
        }

        let port_part = if let Some(port) = caps.get(6) {
            format!(":{}", port.as_str())
        } else {
            "".to_owned()
        };

        let mut range = vec![];
        for d in start..=stop {
            range.push(format!("{}.{}.{}.{}{}", a, b, c, d, port_part));
        }

        return Ok(range);
    } else {
        // check for the port part
        let (cidr_part, port_part) = if expression.contains(":[") && expression.ends_with(']') {
            let (cidr, port) = expression.split_once(":[").unwrap();
            (
                cidr,
                if cidr.contains(':') {
                    // ipv6 cidr
                    format!(":[{}", port)
                } else {
                    // ipv4 cidr
                    format!(":{}", port.trim_end_matches(']'))
                },
            )
        } else {
            (expression, "".to_owned())
        };

        // attempt as cidr
        if let Ok(cidr) = IpCidr::from_str(cidr_part) {
            return Ok(cidr
                .iter()
                .map(|ip| format!("{}{}", ip, port_part))
                .collect());
        }
    }

    Err(format!(
        "could not parse '{}' as a comma separated list of targets or as CIDR",
        expression
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_multiple_targets;

    #[test]
    fn can_parse_comma_separated() {
        let expected = Ok(vec![
            "127.0.0.1:22".to_owned(),
            "www.google.com".to_owned(),
            "cnn.com".to_owned(),
            "8.8.8.8:4444".to_owned(),
        ]);
        let res = parse_multiple_targets("127.0.0.1:22, www.google.com, cnn.com,, 8.8.8.8:4444");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ip_range_without_port() {
        let expected = Ok(vec![
            "192.168.1.1".to_owned(),
            "192.168.1.2".to_owned(),
            "192.168.1.3".to_owned(),
            "192.168.1.4".to_owned(),
            "192.168.1.5".to_owned(),
        ]);
        let res = parse_multiple_targets("192.168.1.1-5");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ip_range_with_port() {
        let expected = Ok(vec![
            "192.168.1.1:1234".to_owned(),
            "192.168.1.2:1234".to_owned(),
            "192.168.1.3:1234".to_owned(),
            "192.168.1.4:1234".to_owned(),
            "192.168.1.5:1234".to_owned(),
        ]);
        let res = parse_multiple_targets("192.168.1.1-5:1234");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ipv4_cidr_without_port() {
        let expected = Ok(vec![
            "192.168.1.0".to_owned(),
            "192.168.1.1".to_owned(),
            "192.168.1.2".to_owned(),
            "192.168.1.3".to_owned(),
        ]);
        let res = parse_multiple_targets("192.168.1.0/30");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ipv4_cidr_with_port() {
        let expected = Ok(vec![
            "192.168.1.0:1234".to_owned(),
            "192.168.1.1:1234".to_owned(),
            "192.168.1.2:1234".to_owned(),
            "192.168.1.3:1234".to_owned(),
        ]);
        let res = parse_multiple_targets("192.168.1.0/30:[1234]");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ipv6_cidr_without_port() {
        let expected = Ok(vec![
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f0".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f1".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f2".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f3".to_owned(),
        ]);
        let res = parse_multiple_targets("2001:4f8:3:ba:2e0:81ff:fe22:d1f1/126");
        assert_eq!(res, expected);
    }

    #[test]
    fn can_parse_ipv6_cidr_with_port() {
        let expected = Ok(vec![
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f0:[1234]".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f1:[1234]".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f2:[1234]".to_owned(),
            "2001:4f8:3:ba:2e0:81ff:fe22:d1f3:[1234]".to_owned(),
        ]);
        let res = parse_multiple_targets("2001:4f8:3:ba:2e0:81ff:fe22:d1f1/126:[1234]");
        assert_eq!(res, expected);
    }
}