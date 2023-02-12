use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    num::ParseIntError,
    str::FromStr,
    time::Duration,
};

use super::model::SessionDescription;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::{alphanumeric1, char, digit1, hex_digit1, one_of},
        streaming::not_line_ending,
    },
    combinator::{all_consuming, cut, map, map_res, opt, recognize},
    error::{convert_error, FromExternalError, ParseError, VerboseError},
    multi::many1,
    sequence::{preceded, terminated, tuple},
    IResult,
};

#[derive(Debug)]
enum SdpNetType {
    IN,
}

#[derive(Debug)]
enum SdpV4OriginAddr {
    Domain(String),
    Addr(Ipv4Addr),
}

#[derive(Debug)]
enum SdpV6OriginAddr {
    Domain(String),
    Addr(Ipv6Addr),
}

#[derive(Debug)]
enum SdpOriginAddr {
    V4(SdpV4OriginAddr),
    V6(SdpV6OriginAddr),
}

#[derive(Debug)]
enum Bandwidth {
    ConferenceTotal(usize),
    ApplicationSpecific(usize),
}

#[derive(Debug)]
enum SdpField {
    Version(usize),
    Origin {
        user_id: Option<String>,
        session_id: i64,
        session_version: i64,
        net_type: SdpNetType,
        addr: SdpOriginAddr,
    },
    SessionName(String),
    SessionInformation(String),
    ConnectionInformation {
        addr: IpAddr,
        ttl: Option<usize>,
        c: Option<usize>,
    },
    Bandwidth(Bandwidth),
    Timing {
        start: u64,
        stop: u64,
    },
    RepeatTimes {
        repeat: Duration,
        active: Duration,
        offset: Duration,
    },
    None,
}

enum Error {
    ParseInt(ParseIntError),
    General,
}

macro_rules! unsigned_int_parser {
    ($fname:tt, $hex_fname:tt, $name:ty) => {
        #[allow(unused)]
        fn $fname<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
            input: &'a str,
        ) -> IResult<&'a str, $name, E> {
            map_res(digit1, |v: &str| {
                v.parse::<$name>().map_err(Error::ParseInt)
            })(input)
        }
        #[allow(unused)]
        fn $hex_fname<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
            input: &'a str,
        ) -> IResult<&'a str, $name, E> {
            preceded(
                opt(alt((tag("0x"), tag("0X")))),
                map_res(recognize(hex_digit1), |s: &str| {
                    <$name>::from_str_radix(s, 16).map_err(Error::ParseInt)
                }),
            )(input)
        }
    };
}

macro_rules! signed_int_parser {
    ($fname:tt, $name:ty) => {
        #[allow(unused)]
        fn $fname<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
            input: &'a str,
        ) -> IResult<&'a str, $name, E> {
            map_res(recognize(preceded(opt(one_of("+-")), digit1)), |s: &str| {
                s.parse::<$name>().map_err(Error::ParseInt)
            })(input)
        }
    };
}

unsigned_int_parser!(dec_u8, hex_u8, u8);
unsigned_int_parser!(dec_u16, hex_u16, u16);
unsigned_int_parser!(dec_u32, hex_u32, u32);
unsigned_int_parser!(dec_u64, hex_u64, u64);
unsigned_int_parser!(dec_usize, hex_usize, usize);
signed_int_parser!(dec_i8, i8);
signed_int_parser!(dec_i16, i16);
signed_int_parser!(dec_i32, i32);
signed_int_parser!(dec_i64, i64);

fn ipv6_addr<'a, E>(input: &'a str) -> IResult<&'a str, Ipv6Addr, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error>,
{
    map_res(recognize(many1(alt((tag(":"), hex_digit1)))), |s: &str| {
        println!("{s:?}");
        s.parse::<Ipv6Addr>().map_err(|_| Error::General)
    })(input)
}

fn ipv4_addr<'a, E>(input: &'a str) -> IResult<&'a str, Ipv4Addr, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error>,
{
    map(
        tuple((
            terminated(dec_u8, char('.')),
            terminated(dec_u8, char('.')),
            terminated(dec_u8, char('.')),
            dec_u8,
        )),
        |(b0, b1, b2, b3)| Ipv4Addr::new(b0, b1, b2, b3),
    )(input)
}

fn domain_name<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    input: &'a str,
) -> IResult<&'a str, String, E> {
    map(
        recognize(many1(terminated(
            many1(alt((alphanumeric1, tag("-")))),
            opt(char('.')),
        ))),
        |v: &str| v.to_owned(),
    )(input)
}

fn sdp_origin_addr<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    input: &'a str,
) -> IResult<&'a str, SdpOriginAddr, E> {
    alt((
        preceded(
            tag("IP4 "),
            alt((
                map(ipv4_addr, |addr| {
                    SdpOriginAddr::V4(SdpV4OriginAddr::Addr(addr))
                }),
                map(domain_name, |name| {
                    SdpOriginAddr::V4(SdpV4OriginAddr::Domain(name))
                }),
            )),
        ),
        preceded(
            tag("IP6 "),
            alt((
                map(domain_name, |name| {
                    SdpOriginAddr::V6(SdpV6OriginAddr::Domain(name))
                }),
                map(ipv6_addr, |addr| {
                    SdpOriginAddr::V6(SdpV6OriginAddr::Addr(addr))
                }),
            )),
        ),
    ))(input)
}

fn line<'a, F: 'a, O, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    terminated(inner, alt((tag("\r\n"), tag("\n"))))
}

fn sdp_field<'a, F, E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a>(
    field_id: &'static str,
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, SdpField, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, SdpField, E> + 'a,
{
    preceded(tag(field_id), preceded(char('='), cut(inner)))
}

fn sdp_string_field<'a, F, E>(
    field_id: &'static str,
    mut inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
    F: FnMut(String) -> SdpField + 'a,
{
    sdp_field(
        field_id,
        map(not_line_ending, move |s: &str| inner(s.to_string())),
    )
}

fn version_field<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a>(
    input: &'a str,
) -> IResult<&'a str, SdpField, E> {
    sdp_field("v", map(dec_usize, SdpField::Version))(input)
}

fn user_id<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    input: &'a str,
) -> IResult<&'a str, Option<String>, E> {
    alt((
        map(tag("-"), |_| None),
        map(alphanumeric1, |s: &str| Some(s.to_string())),
    ))(input)
}

fn origin_net_type<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    input: &'a str,
) -> IResult<&'a str, SdpNetType, E> {
    map(tag("IN"), |_| SdpNetType::IN)(input)
}

fn origin_field<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a>(
    input: &'a str,
) -> IResult<&'a str, SdpField, E> {
    sdp_field(
        "o",
        map(
            tuple((
                terminated(user_id, char(' ')),
                terminated(dec_i64, char(' ')),
                terminated(dec_i64, char(' ')),
                terminated(origin_net_type, char(' ')),
                sdp_origin_addr,
            )),
            |(user_id, session_id, session_version, net_type, addr)| SdpField::Origin {
                user_id,
                session_id,
                session_version,
                net_type,
                addr,
            },
        ),
    )(input)
}

fn con_info_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_field(
        "c",
        map(
            tuple((
                tag("IN "),
                alt((
                    map(tuple((tag("IP4 "), ipv4_addr)), |(_, a)| IpAddr::V4(a)),
                    map(tuple((tag("IP6 "), ipv6_addr)), |(_, a)| IpAddr::V6(a)),
                )),
                opt(preceded(tag("/"), dec_usize)),
                opt(preceded(tag("/"), dec_usize)),
            )),
            |(_, addr, ttl, c)| SdpField::ConnectionInformation { addr, ttl, c },
        ),
    )(input)
}

fn bandwidth_field_impl<'a, F, E>(
    t: &'static str,
    mut f: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, SdpField, E> + 'a
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
    F: FnMut(usize) -> Bandwidth + 'a,
{
    preceded(
        tag(t),
        preceded(tag(":"), map(dec_usize, move |s| SdpField::Bandwidth(f(s)))),
    )
}

fn bandwidth_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_field(
        "b",
        alt((
            bandwidth_field_impl("AS", Bandwidth::ApplicationSpecific),
            bandwidth_field_impl("CT", Bandwidth::ConferenceTotal),
        )),
    )(input)
}

fn session_name_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_string_field("s", SdpField::SessionName)(input)
}

fn session_information_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_string_field("i", SdpField::SessionInformation)(input)
}

fn timing_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_field(
        "t",
        map(tuple((dec_u64, tag(" "), dec_u64)), |(start, _, stop)| {
            SdpField::Timing { start, stop }
        }),
    )(input)
}

fn time_value<'a, E>(input: &'a str) -> IResult<&'a str, Duration, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    map(
        tuple((
            dec_u64,
            opt(alt((char('d'), char('h'), char('m'), char('s')))),
        )),
        |(v, tc)| match tc {
            Some(c) => match c {
                'd' => Duration::from_secs(v * 86400),
                'h' => Duration::from_secs(v * 3600),
                'm' => Duration::from_secs(v * 60),
                's' => Duration::from_secs(v),
                _ => panic!(),
            },
            None => Duration::from_secs(v),
        },
    )(input)
}

fn repeat_field<'a, E>(input: &'a str) -> IResult<&'a str, SdpField, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a,
{
    sdp_field(
        "r",
        map(
            tuple((
                terminated(time_value, char(' ')),
                terminated(time_value, char(' ')),
                time_value,
            )),
            |(repeat, active, offset)| SdpField::RepeatTimes {
                repeat,
                active,
                offset,
            },
        ),
    )(input)
}

fn empty<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error>>(
    input: &'a str,
) -> IResult<&'a str, SdpField, E> {
    cut(map(tag(""), |_| SdpField::None))(input)
}

fn sdp<'a, E: ParseError<&'a str> + FromExternalError<&'a str, Error> + 'a>(
    input: &'a str,
) -> IResult<&'a str, Vec<SdpField>, E> {
    all_consuming(many1(line(cut(alt((
        version_field,
        origin_field,
        session_name_field,
        session_information_field,
        con_info_field,
        bandwidth_field,
        timing_field,
        repeat_field,
        empty,
    ))))))(input)
}

#[test]
fn test_hex() {
    let str = "ff02::1:ff00:0";

    println!("{:?}", ipv6_addr::<VerboseError<_>>(str));
}

#[test]
fn test() {
    let str = "v=1
o=- 1 1 IN IP4 83.131.231.21
s=Hi! this is a string\r
i=This is some session information\r
c=IN IP4 224.2.36.42/127/4
b=CT:1234
t=1000 1000
r=1d 20m 0s
";

    let str2 = "v=0
o=- 108902 53 IN IP4 10.0.81.53
s=jpeg-xs-test-session
i=jpeg-xs Testing 238.10.0.29:20000
t=0 0
c=IN IP4 238.10.0.29/32
b=AS:4396
";

    match sdp::<VerboseError<_>>(str2) {
        Ok(v) => {
            println!("{v:#?}");
        }
        Err(e) => match e {
            nom::Err::Incomplete(incomplete) => {
                println!("{incomplete:?}");
            }
            nom::Err::Error(err) => {
                println!("err: {}", convert_error(str2, err))
            }
            nom::Err::Failure(f) => {
                println!("fail: {}", convert_error(str2, f))
            }
        },
    }
}

#[test]
fn test_time_value() {
    let str = "1";

    println!("{:?}", time_value::<VerboseError<_>>(str).unwrap().1);
}
