use core::fmt::Display;

impl<'a> Display for super::Ipv4PacketView<'a> {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            writeln!(f, "IPv4Packet ({} bytes) {{", self.total_len())?;
            writeln!(f, "    IPv4Header ({} bytes) {{", self.header_len())?;
            writeln!(f, "        Addresses: {} -> {}",
                self.source_addr(),
                self.dest_addr()
            )?;
            writeln!(f, "        Protocol: {}", self.protocol())?;
            writeln!(f, "        Contol: [DSCP: {}] [ECN: {}]",
                self.dscp(),
                self.ecn()
            )?;
            writeln!(f, "        Fragmentation: [Identification: {:#06x}] [DF: {}] [MF: {}] [Offset: {}]",
                self.identification(),
                self.df(),
                self.mf(),
                self.offset()
            )?;
            writeln!(f, "        TTL: {}", self.ttl())?;
            writeln!(f, "        Checksum: {:#06x}", self.checksum())?;
            writeln!(f, "    }}")?;
            match self.options() {
                Ok(opts) => {
                    writeln!(f, "    Ipv4Options ({} bytes) {{", opts.len())?;
                    writeln!(f, "        [ignored]")?;
                    writeln!(f, "    }}")?;
                }
                Err(e) => {
                    writeln!(f, "    Ipv4Options (? bytes) {{")?;
                    writeln!(f, "        [broken: {}]", e)?;
                    writeln!(f, "    }}")?;
                }
            }

            write!(f, "}}")
        } else {
            write!(
                f,
                "Ipv4Packet {{ [{} -> {}] [{} bytes] [{} bytes header + {} bytes payload] }}",
                self.source_addr(),
                self.dest_addr(),
                self.total_len(),
                self.header_len(),
                self.total_len() as i64 - self.header_len() as i64
            )
        }
    }
}
