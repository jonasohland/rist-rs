extern crate librist_sys;
use std::ffi::{CStr, CString, NulError};
use std::str::Utf8Error;

pub struct Config {
    rist_peer_config: Box<librist_sys::rist_peer_config>,
}

#[derive(PartialEq, Debug)]
pub enum RecoveryMode {
    Unconfigured,
    Disabled,
    Time,
}

#[derive(PartialEq, Debug)]
pub enum CongestionControlMode {
    Off,
    Normal,
    Aggressive,
}

#[derive(PartialEq, Debug)]
pub enum TimingMode {
    Source,
    Arrival,
    Rtc,
}

fn rist_peer_config_alloc() -> Box<librist_sys::rist_peer_config> {
    unsafe {
        let mut rist_peer_config = Box::new(std::mem::zeroed::<librist_sys::rist_peer_config>());
        librist_sys::rist_peer_config_defaults_set(
            &mut *rist_peer_config as *mut librist_sys::rist_peer_config,
        );
        rist_peer_config
    }
}

fn rist_set_short_c_string(str: &str, c_str: &mut [i8; 128]) -> std::result::Result<(), NulError> {
    let src_c_str = CString::new(str)?;
    let src_ptr = src_c_str.as_ptr();
    unsafe {
        std::ptr::copy_nonoverlapping(
            src_ptr,
            &mut *c_str as *mut i8,
            std::cmp::max(str.len() + 1, 128),
        )
    }
    Ok(())
}

fn rist_set_long_c_string(str: &str, c_str: &mut [i8; 256]) -> std::result::Result<(), NulError> {
    let src_c_str = CString::new(str)?;
    let src_ptr = src_c_str.as_ptr();
    unsafe {
        std::ptr::copy_nonoverlapping(
            src_ptr,
            &mut *c_str as *mut i8,
            std::cmp::max(str.len() + 1, 256),
        )
    }
    Ok(())
}

fn rist_get_long_c_string(c_str: &[i8; 256]) -> std::result::Result<&str, Utf8Error> {
    unsafe { CStr::from_ptr(c_str as *const i8) }.to_str()
}

fn rist_get_short_c_string(c_str: &[i8; 128]) -> std::result::Result<&str, Utf8Error> {
    unsafe { CStr::from_ptr(c_str as *const i8) }.to_str()
}

fn rist_recovery_mode_from_native(mode: RecoveryMode) -> librist_sys::rist_recovery_mode {
    match mode {
        RecoveryMode::Unconfigured => {
            librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_UNCONFIGURED
        }
        RecoveryMode::Disabled => librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_DISABLED,
        RecoveryMode::Time => librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_TIME,
    }
}

fn rist_recovery_mode_to_native(mode: librist_sys::rist_recovery_mode) -> Option<RecoveryMode> {
    match mode {
        librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_UNCONFIGURED => {
            Some(RecoveryMode::Unconfigured)
        }
        librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_DISABLED => Some(RecoveryMode::Disabled),
        librist_sys::rist_recovery_mode_RIST_RECOVERY_MODE_TIME => Some(RecoveryMode::Time),
        _ => None,
    }
}

fn rist_congestion_control_mode_from_native(
    mode: CongestionControlMode,
) -> librist_sys::rist_congestion_control_mode {
    match mode {
        CongestionControlMode::Off => {
            librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_OFF
        }
        CongestionControlMode::Normal => {
            librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_NORMAL
        }
        CongestionControlMode::Aggressive => {
            librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_AGGRESSIVE
        }
    }
}

fn rist_congestion_control_mode_to_native(
    mode: librist_sys::rist_congestion_control_mode,
) -> Option<CongestionControlMode> {
    match mode {
        librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_OFF => {
            Some(CongestionControlMode::Off)
        }
        librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_NORMAL => {
            Some(CongestionControlMode::Normal)
        }
        librist_sys::rist_congestion_control_mode_RIST_CONGESTION_CONTROL_MODE_AGGRESSIVE => {
            Some(CongestionControlMode::Aggressive)
        }
        _ => None,
    }
}

fn rist_timing_mode_to_native(mode: librist_sys::rist_timing_mode) -> Option<TimingMode> {
    match mode {
        librist_sys::rist_timing_mode_RIST_TIMING_MODE_SOURCE => Some(TimingMode::Source),
        librist_sys::rist_timing_mode_RIST_TIMING_MODE_ARRIVAL => Some(TimingMode::Arrival),
        librist_sys::rist_timing_mode_RIST_TIMING_MODE_RTC => Some(TimingMode::Rtc),
        _ => None,
    }
}

fn rist_timing_mode_from_native(mode: TimingMode) -> librist_sys::rist_timing_mode {
    match mode {
        TimingMode::Source => librist_sys::rist_timing_mode_RIST_TIMING_MODE_SOURCE,
        TimingMode::Arrival => librist_sys::rist_timing_mode_RIST_TIMING_MODE_ARRIVAL,
        TimingMode::Rtc => librist_sys::rist_timing_mode_RIST_TIMING_MODE_RTC,
    }
}

impl Config {
    pub fn new() -> Config {
        let mut config = Config {
            rist_peer_config: rist_peer_config_alloc(),
        };
        unsafe {
            librist_sys::rist_peer_config_defaults_set(
                &mut *config.rist_peer_config as *mut librist_sys::rist_peer_config,
            );
        }
        return config;
    }

    pub fn from_url_str(url: &str) -> Result<Config, &str> {
        let mut cfg = Config::new();
        CString::new(url)
            .map_err(|_| "null error")
            .and_then(|c_url| {
                if unsafe {
                    let mut ptr = cfg.mut_rist_peer_config_ptr();
                    librist_sys::rist_parse_address2(
                        c_url.as_ptr(),
                        &mut ptr as *mut *mut librist_sys::rist_peer_config,
                    )
                } != 0
                {
                    Err("failed to parse url string")
                } else {
                    Ok(cfg)
                }
            })
    }

    pub fn mut_rist_peer_config_ptr(&mut self) -> *mut librist_sys::rist_peer_config {
        &mut *self.rist_peer_config as *mut librist_sys::rist_peer_config
    }

    pub fn rist_peer_config_ptr(&self) -> *const librist_sys::rist_peer_config {
        &*self.rist_peer_config as *const librist_sys::rist_peer_config
    }

    pub fn get_virt_dest_port(&self) -> u16 {
        self.rist_peer_config.virt_dst_port
    }

    pub fn set_virt_dest_port(&mut self, port: u16) -> &mut Config {
        self.rist_peer_config.virt_dst_port = port;
        self
    }

    pub fn get_recovery_max_bitrate(&self) -> u32 {
        self.rist_peer_config.recovery_maxbitrate
    }

    pub fn set_recovery_max_bitrate(&mut self, rate: u32) -> &mut Config {
        (self.rist_peer_config).recovery_maxbitrate = rate;
        self
    }

    pub fn get_recovery_max_bitrate_return(&self) -> u32 {
        self.rist_peer_config.recovery_maxbitrate_return
    }

    pub fn set_recovery_max_bitrate_return(&mut self, rate: u32) -> &mut Config {
        (self.rist_peer_config).recovery_maxbitrate_return = rate;
        self
    }

    pub fn get_recovery_length(&self) -> (u32, u32) {
        (
            self.rist_peer_config.recovery_length_min,
            self.rist_peer_config.recovery_length_max,
        )
    }

    pub fn set_recovery_length(&mut self, length_min_max_ms: (u32, u32)) {
        self.rist_peer_config.recovery_length_min = length_min_max_ms.0;
        self.rist_peer_config.recovery_length_max = length_min_max_ms.1;
    }

    pub fn get_recovery_reorder_buffer_length(&self) -> u32 {
        self.rist_peer_config.recovery_reorder_buffer
    }

    pub fn set_recovery_reorder_buffer_length(&mut self, length_ms: u32) -> &mut Config {
        self.rist_peer_config.recovery_reorder_buffer = length_ms;
        self
    }

    pub fn get_recovery_mode(&self) -> Result<RecoveryMode, &str> {
        rist_recovery_mode_to_native(self.rist_peer_config.recovery_mode).ok_or("")
    }

    pub fn set_recovery_mode(&mut self, mode: RecoveryMode) -> &mut Config {
        self.rist_peer_config.recovery_mode = rist_recovery_mode_from_native(mode);
        self
    }

    pub fn get_rtt(&self) -> (u32, u32) {
        (
            self.rist_peer_config.recovery_rtt_min,
            self.rist_peer_config.recovery_rtt_max,
        )
    }

    pub fn set_rtt(&mut self, range: (u32, u32)) -> &mut Config {
        self.rist_peer_config.recovery_rtt_min = range.0;
        self.rist_peer_config.recovery_rtt_max = range.1;
        self
    }

    pub fn set_weight(&mut self, weight: u32) -> &mut Config {
        self.rist_peer_config.weight = weight;
        self
    }

    pub fn get_weight(&self) -> u32 {
        self.rist_peer_config.weight
    }

    pub fn get_secret_len(&self) -> Result<usize, &str> {
        let len_signed = self.rist_peer_config.key_size;
        if len_signed < 0 || len_signed > librist_sys::RIST_MAX_STRING_LONG as i32 {
            Err("Invalid key size")
        } else {
            Ok(len_signed as usize)
        }
    }

    pub fn get_secret(&self) -> Result<Vec<i8>, &str> {
        self.get_secret_len().map(|len| {
            let mut v = std::vec::Vec::<i8>::new();
            v.extend_from_slice(&self.rist_peer_config.secret);
            v.resize(len, 0);
            v
        })
    }

    pub fn get_secret_as_slice(&self) -> Result<&[i8], &str> {
        self.get_secret_len().map(|len| unsafe {
            std::slice::from_raw_parts(&self.rist_peer_config.secret as *const i8, len)
        })
    }

    pub fn set_secret(&mut self, secret: &[i8]) -> Result<&mut Config, &str> {
        if secret.len() > librist_sys::RIST_MAX_STRING_LONG as usize {
            Err("secret too long")
        } else {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    secret.as_ptr(),
                    &mut self.rist_peer_config.secret as *mut i8,
                    secret.len(),
                );
                self.rist_peer_config.key_size = secret.len() as i32;
            }
            Ok(self)
        }
    }

    pub fn get_key_rotation(&self) -> u32 {
        self.rist_peer_config.key_rotation
    }

    pub fn set_key_rotation(&mut self, key_rotation: u32) -> &mut Config {
        self.rist_peer_config.key_rotation = key_rotation;
        self
    }

    pub fn get_compression(&self) -> bool {
        self.rist_peer_config.compression > 0
    }

    pub fn set_compression(&mut self, enabled: bool) -> &mut Config {
        self.rist_peer_config.compression = enabled as i32;
        self
    }

    pub fn get_cname(&self) -> Option<&str> {
        rist_get_short_c_string(&self.rist_peer_config.cname).ok()
    }

    pub fn set_cname(&mut self, cname: &str) -> Result<&mut Config, &str> {
        rist_set_short_c_string(cname, &mut self.rist_peer_config.cname)
            .map_err(|_| "null error")
            .map(|_| self)
    }

    pub fn get_congestion_control_mode(&self) -> Result<CongestionControlMode, &str> {
        rist_congestion_control_mode_to_native(self.rist_peer_config.congestion_control_mode)
            .ok_or("err")
    }

    pub fn set_congestion_control_mode(&mut self, mode: CongestionControlMode) -> &mut Config {
        self.rist_peer_config.congestion_control_mode =
            rist_congestion_control_mode_from_native(mode);
        self
    }

    pub fn get_retries(&self) -> (u32, u32) {
        (
            self.rist_peer_config.min_retries,
            self.rist_peer_config.max_retries,
        )
    }

    pub fn set_retries(&mut self, min_max_retries: (u32, u32)) -> &mut Config {
        self.rist_peer_config.min_retries = min_max_retries.0;
        self.rist_peer_config.max_retries = min_max_retries.1;
        self
    }

    pub fn get_session_timeout(&self) -> u32 {
        self.rist_peer_config.session_timeout
    }

    pub fn set_session_timeout(&mut self, timeout: u32) -> &mut Config {
        self.rist_peer_config.session_timeout = timeout;
        self
    }

    pub fn get_keepalive_interval(&self) -> u32 {
        self.rist_peer_config.keepalive_interval
    }

    pub fn set_keepalive_interval(&mut self, interval: u32) -> &mut Config {
        self.rist_peer_config.keepalive_interval = interval;
        self
    }

    pub fn get_timing_mode(&self) -> Result<TimingMode, &str> {
        rist_timing_mode_to_native(self.rist_peer_config.timing_mode).ok_or("err")
    }

    pub fn set_timing_mode(&mut self, mode: TimingMode) -> &mut Config {
        self.rist_peer_config.timing_mode = rist_timing_mode_from_native(mode);
        self
    }

    pub fn get_srp_username(&self) -> Option<&str> {
        rist_get_long_c_string(&self.rist_peer_config.srp_username).ok()
    }

    pub fn set_srp_username(&mut self, srp_username: &str) -> Result<&mut Config, &str> {
        rist_set_long_c_string(srp_username, &mut self.rist_peer_config.srp_username)
            .map_err(|_| "null error")
            .map(|_| self)
    }

    pub fn get_srp_password(&self) -> Option<&str> {
        rist_get_long_c_string(&self.rist_peer_config.srp_password).ok()
    }

    pub fn set_srp_password(&mut self, srp_password: &str) -> Result<&mut Config, &str> {
        rist_set_long_c_string(srp_password, &mut self.rist_peer_config.srp_password)
            .map_err(|_| "null error")
            .map(|_| self)
    }

    pub fn set_srp_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<&mut Config, &str> {
        self.set_srp_username(username)
            .and_then(|cfg| cfg.set_srp_password(password))
    }
}

#[cfg(test)]
mod tests {
    use crate::peer::{CongestionControlMode, RecoveryMode, TimingMode};

    #[test]
    fn defaults() {
        let cfg = super::Config::new();
        assert_eq!(
            cfg.get_virt_dest_port() as u32,
            librist_sys::RIST_DEFAULT_VIRT_DST_PORT
        );
        assert_eq!(cfg.get_recovery_mode().unwrap(), RecoveryMode::Time);
        assert_eq!(
            cfg.get_recovery_max_bitrate(),
            librist_sys::RIST_DEFAULT_RECOVERY_MAXBITRATE
        );
        assert_eq!(
            cfg.get_recovery_max_bitrate_return(),
            librist_sys::RIST_DEFAULT_RECOVERY_MAXBITRATE_RETURN
        );
        assert_eq!(
            cfg.get_recovery_length().0,
            librist_sys::RIST_DEFAULT_RECOVERY_LENGHT_MIN
        );
        assert_eq!(
            cfg.get_recovery_length().1,
            librist_sys::RIST_DEFAULT_RECOVERY_LENGHT_MAX
        );
        assert_eq!(
            cfg.get_recovery_reorder_buffer_length(),
            librist_sys::RIST_DEFAULT_RECOVERY_REORDER_BUFFER
        );
        assert_eq!(cfg.get_rtt().0, librist_sys::RIST_DEFAULT_RECOVERY_RTT_MIN);
        assert_eq!(cfg.get_rtt().1, librist_sys::RIST_DEFAULT_RECOVERY_RTT_MAX);
        assert_eq!(cfg.get_weight(), 0);
        assert_eq!(cfg.get_secret().unwrap(), []);
        assert_eq!(cfg.get_secret_as_slice().unwrap(), []);
        // TODO make sure this is default-initialized?
        assert_eq!(cfg.get_key_rotation(), 0);
        assert_eq!(cfg.get_compression(), false);
        assert_eq!(cfg.get_cname().unwrap(), "");
        assert_eq!(
            cfg.get_congestion_control_mode().unwrap(),
            CongestionControlMode::Normal
        );
        assert_eq!(cfg.get_retries().0, librist_sys::RIST_DEFAULT_MIN_RETRIES);
        assert_eq!(cfg.get_retries().1, librist_sys::RIST_DEFAULT_MAX_RETRIES);

        // TODO: BUG! this is not set corretly by the library
        assert_eq!(
            cfg.get_session_timeout(),
            /* librist_sys::RIST_DEFAULT_SESSION_TIMEOUT */ 0
        );
        assert_eq!(
            cfg.get_keepalive_interval(),
            /* librist_sys::RIST_DEFAULT_KEEPALIVE_INTERVAL */ 0
        );
        assert_eq!(cfg.get_timing_mode().unwrap(), TimingMode::Source);
        assert_eq!(cfg.get_srp_username().unwrap(), "");
        assert_eq!(cfg.get_srp_password().unwrap(), "");
    }

    #[test]
    fn virt_dest_port() {
        let mut cfg = super::Config::new();
        cfg.set_virt_dest_port(19);
        assert_eq!(cfg.get_virt_dest_port(), 19);
    }

    #[test]
    fn recovery_mode() {
        let mut cfg = super::Config::new();
        cfg.set_recovery_mode(RecoveryMode::Disabled);
        assert_eq!(cfg.get_recovery_mode().unwrap(), RecoveryMode::Disabled);
    }

    #[test]
    fn recovery_max_bitrate() {
        let mut cfg = super::Config::new();
        cfg.set_recovery_max_bitrate(2292);
        assert_eq!(cfg.get_recovery_max_bitrate(), 2292);
    }

    #[test]
    fn recovery_max_bitrate_return() {
        let mut cfg = super::Config::new();
        cfg.set_recovery_max_bitrate_return(5595);
        assert_eq!(cfg.get_recovery_max_bitrate_return(), 5595);
    }

    #[test]
    fn recovery_length() {
        let mut cfg = super::Config::new();
        cfg.set_recovery_length((22, 2222));
        assert_eq!(cfg.get_recovery_length(), (22, 2222));
    }

    #[test]
    fn recovery_reorder_buffer() {
        let mut cfg = super::Config::new();
        cfg.set_recovery_reorder_buffer_length(3323);
        assert_eq!(cfg.get_recovery_reorder_buffer_length(), 3323);
    }

    #[test]
    fn rtt() {
        let mut cfg = super::Config::new();
        cfg.set_rtt((52, 993));
        assert_eq!(cfg.get_rtt().0, 52);
        assert_eq!(cfg.get_rtt().1, 993);
    }

    #[test]
    fn weight() {
        let mut cfg = super::Config::new();
        cfg.set_weight(3);
        assert_eq!(cfg.get_weight(), 3);
    }

    #[test]
    fn secret() {
        let mut cfg = super::Config::new();
        cfg.set_secret(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(cfg.get_secret().unwrap(), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn secret_slice() {
        let mut cfg = super::Config::new();
        cfg.set_secret(&[1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(cfg.get_secret_as_slice().unwrap(), [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn key_rotation() {
        let mut cfg = super::Config::new();
        cfg.set_key_rotation(39);
        assert_eq!(cfg.get_key_rotation(), 39);
    }

    #[test]
    fn compression() {
        let mut cfg = super::Config::new();
        cfg.set_compression(true);
        assert_eq!(cfg.get_compression(), true);
    }

    #[test]
    fn cname() {
        let mut cfg = super::Config::new();
        cfg.set_cname("test_name").unwrap();
        assert_eq!(cfg.get_cname().unwrap(), "test_name");
    }

    #[test]
    fn congestion_control_mode() {
        let mut cfg = super::Config::new();
        cfg.set_congestion_control_mode(CongestionControlMode::Aggressive);
        assert_eq!(
            cfg.get_congestion_control_mode().unwrap(),
            CongestionControlMode::Aggressive
        );
    }

    #[test]
    fn retries() {
        let mut cfg = super::Config::new();
        cfg.set_retries((0, 33));
        assert_eq!(cfg.get_retries(), (0, 33));
    }

    #[test]
    fn session_timeout() {
        let mut cfg = super::Config::new();
        cfg.set_session_timeout(9943);
        assert_eq!(cfg.get_session_timeout(), 9943);
    }

    #[test]
    fn keepalive_interval() {
        let mut cfg = super::Config::new();
        cfg.set_keepalive_interval(3853);
        assert_eq!(cfg.get_keepalive_interval(), 3853);
    }

    #[test]
    fn timing_mode() {
        let mut cfg = super::Config::new();
        cfg.set_timing_mode(TimingMode::Rtc);
        assert_eq!(cfg.get_timing_mode().unwrap(), TimingMode::Rtc);
    }

    #[test]
    fn srp_username() {
        let mut cfg = super::Config::new();
        cfg.set_srp_username("test-username").unwrap();
        assert_eq!(cfg.get_srp_username().unwrap(), "test-username");
    }

    #[test]
    fn srp_password() {
        let mut cfg = super::Config::new();
        cfg.set_srp_password("test-password").unwrap();
        assert_eq!(cfg.get_srp_password().unwrap(), "test-password");
    }

    #[test]
    fn from_string_weight() {
        let cfg = super::Config::from_url_str("rist://127.0.0.1:8838?weight=8838").unwrap();
        assert_eq!(cfg.get_weight(), 8838);
    }

    #[test]
    fn from_string_invalid() {
        let cfg = super::Config::from_url_str("rist://@127.0.0.1:8838?foo=bar");
        assert_eq!(cfg.err().unwrap(), "failed to parse url string");
    }
}
