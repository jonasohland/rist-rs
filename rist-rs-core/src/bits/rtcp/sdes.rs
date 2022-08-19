use core::str::from_utf8;

pub mod error {
    use core::str::Utf8Error;

    /// Error returned by the SDES item iterator
    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        /// SDES item contains invalid utf8 characters
        Utf8(Utf8Error),
        /// End of packet reached while parsing items
        EndOfPacketReached,
        /// Padding contains a non-null character or is otherwise invalid
        InvalidPadding,
        /// Unknown item type
        UnknownType,
    }

    /// Implemented to short-circuit convert UTF8 errors to the parser error type
    impl From<Utf8Error> for Error {
        fn from(e: Utf8Error) -> Self {
            Self::Utf8(e)
        }
    }
}

/// A source description item
#[derive(Debug, Clone, Copy)]
pub struct SourceDescriptionItem<'a> {
    /// SSRC of the corresponding source
    pub ssrc: u32,

    /// The payload of the item
    pub payload: SourceDescriptionItemPayload<'a>,
}

/// Types of source description payloads an item may contain. For RIST only the CNAME payload type
/// is relevant.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum SourceDescriptionItemPayload<'a> {
    CNAME(&'a str),
    NAME(&'a str),
    EMAIL(&'a str),
    PHONE(&'a str),
    LOC(&'a str),
    TOOL(&'a str),
    NOTE(&'a str),
    PRIV(&'a str),
}

impl<'a> SourceDescriptionItem<'a> {
    pub fn try_new<T, U>(ssrc: u32, desc_type: u8, bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data: &'a [u8] = bytes.as_ref().into();
        match desc_type {
            1 => Ok(SourceDescriptionItemPayload::CNAME(from_utf8(data)?)),
            2 => Ok(SourceDescriptionItemPayload::NAME(from_utf8(data)?)),
            3 => Ok(SourceDescriptionItemPayload::EMAIL(from_utf8(data)?)),
            4 => Ok(SourceDescriptionItemPayload::PHONE(from_utf8(data)?)),
            5 => Ok(SourceDescriptionItemPayload::LOC(from_utf8(data)?)),
            6 => Ok(SourceDescriptionItemPayload::TOOL(from_utf8(data)?)),
            7 => Ok(SourceDescriptionItemPayload::NOTE(from_utf8(data)?)),
            8 => Ok(SourceDescriptionItemPayload::PRIV(from_utf8(data)?)),
            _ => Err(error::Error::UnknownType),
        }
        .map(|payload| SourceDescriptionItem { ssrc, payload })
    }
}

impl<'a> TryFrom<(u32, u8, &'a [u8])> for SourceDescriptionItem<'a> {
    type Error = error::Error;
    fn try_from(value: (u32, u8, &'a [u8])) -> Result<Self, Self::Error> {
        let (ssrc, t, data) = value;
        SourceDescriptionItem::try_new(ssrc, t, data)
    }
}

/// length in bytes of the ssrc field in each source description item
const SSRC_LEN: usize = 4;

/// length of the item header (item type + length)
const ITEM_HEADER_LEN: usize = 2;

/// minimum allowed length of a source description item
const ITEM_MIN_LEN: usize = SSRC_LEN + ITEM_HEADER_LEN + 2;

/// offset of the item length field
const ITEM_LEN_OFFSET: usize = SSRC_LEN + 1;

/// offset of the item type field
const ITEM_TYPE_OFFSET: usize = SSRC_LEN;

/// offset of the item payload
const ITEM_OFFSET: usize = SSRC_LEN + ITEM_HEADER_LEN;

/// Iterate over a list of SDES item chunks in an rtcp packet.
/// Parses the Items on-the-fly and stops emitting elements as soon as the end of data
/// is reached or an error occurs
#[derive(Debug, Clone, Copy)]
pub struct SourceDescriptionMessageIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> SourceDescriptionMessageIterator<'a> {
    /// Creates a new [SourceDescriptionMessageIterator] from a reference to something
    /// that can be converted into a reference to a slice of bytes.
    /// Returns an error if no items can be possibly parsed from the slice of the given length.
    pub fn try_new<T, U>(bytes: &'a T) -> Result<Self, error::Error>
    where
        T: AsRef<U> + ?Sized,
        U: ?Sized + 'a,
        &'a U: Into<&'a [u8]>,
    {
        let data = bytes.as_ref().into();
        // empty slice is allowed (no items), if not empty: need at least
        // ITEM_MIN_LEN bytes of data
        if !data.is_empty() && data.len() < ITEM_MIN_LEN {
            Err(error::Error::EndOfPacketReached)
        } else {
            Ok(SourceDescriptionMessageIterator { data, pos: 0 })
        }
    }
}

/// Implemented to create an [SourceDescriptionMessageIterator] from a slice of data.
/// Returns an error if no items could be successfully parsed from the slice
impl<'a> TryFrom<&'a [u8]> for SourceDescriptionMessageIterator<'a> {
    type Error = error::Error;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_new(data)
    }
}

#[cfg(debug_assertions)]
fn check_padding(padding: &[u8]) {
    for p in padding {
        debug_assert_eq!(*p, 0x00);
    }
}

impl<'a> SourceDescriptionMessageIterator<'a> {
    fn next_impl(&self) -> Result<(usize, SourceDescriptionItem<'a>), error::Error> {
        if self.pos + ITEM_MIN_LEN >= self.data.len() {
            Err(error::Error::EndOfPacketReached)
        } else {
            let ssrc = u32::from_be_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);

            // length of the sdes item as reported by the indicating field
            let item_len = self.data[self.pos + ITEM_LEN_OFFSET] as usize;

            // expect to be padded to 32bit boundary
            let padding = match (item_len + ITEM_HEADER_LEN) % 4 {
                0 => 4,
                p => 4 - p,
            };

            // index of first and last item bytes
            let start_of_item = self.pos + ITEM_OFFSET;
            let end_of_item = self.pos + ITEM_OFFSET + item_len;

            // end of padding added
            let end_of_padding = end_of_item + padding;

            if self.data.len() < end_of_padding {
                Err(error::Error::EndOfPacketReached)
            } else {
                #[cfg(debug_assertions)]
                check_padding(&self.data[end_of_item..end_of_padding]);
                Ok((
                    SSRC_LEN + ITEM_HEADER_LEN + item_len + padding, // offset to next chunk
                    SourceDescriptionItem::try_from((
                        ssrc,
                        self.data[self.pos + ITEM_TYPE_OFFSET], // item type field
                        &self.data[start_of_item..end_of_item], // item data
                    ))?,
                ))
            }
        }
    }
}

impl<'a> Iterator for SourceDescriptionMessageIterator<'a> {
    type Item = Result<SourceDescriptionItem<'a>, error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.data.len() || self.pos == usize::MAX {
            None
        } else {
            match self.next_impl() {
                Ok((offset, item)) => {
                    debug_assert_eq!(offset % 4, 0);
                    self.pos += offset;
                    Some(Ok(item))
                }
                Err(e) => {
                    // an error occurred, no more items can be returned by this iterator
                    self.pos = usize::MAX;
                    Some(Err(e))
                }
            }
        }
    }
}

mod test {
    use crate::bits::rtcp::sdes::SourceDescriptionItemPayload;

    #[test]
    fn empty() {
        let iterator = super::SourceDescriptionMessageIterator::try_from([].as_slice()).unwrap();
        assert_eq!(iterator.count(), 0);
    }

    /// 1 CNAME SDES without padding
    const BASIC_NO_PADDING: [u8; 40] = [
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1e, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x6c, 0x00, 0x00, 0x00, 0x00, // sdes 3 ->
    ];

    #[test]
    fn basic_cname_no_padding() {
        let iter =
            super::SourceDescriptionMessageIterator::try_from(BASIC_NO_PADDING.as_slice()).unwrap();
        let items = iter.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 1);
        if let SourceDescriptionItemPayload::CNAME(name) = items[0].payload {
            assert_eq!("jonasohland-macbook-pro.locall", name);
            assert_eq!(items[0].ssrc, 0x1d56bc2e);
        } else {
            panic!("wrong item returned as payload")
        }
    }

    const BASIC_WITH_PADDING: [u8; 36] = [
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1d, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x00, // sdes 2 ->
    ];

    // 1 CNAME SDES with padding
    #[test]
    fn basic_cname_with_padding() {
        let iter = super::SourceDescriptionMessageIterator::try_from(BASIC_WITH_PADDING.as_slice())
            .unwrap();
        let items = iter.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 1);
        if let SourceDescriptionItemPayload::CNAME(name) = items[0].payload {
            assert_eq!("jonasohland-macbook-pro.local", name);
            assert_eq!(items[0].ssrc, 0x1d56bc2e);
        } else {
            panic!("wrong item returned as payload")
        }
    }

    // 2 CNAME SDES items one with and one without padding
    const MULTI_WITH_AND_WITHOUT_PADDING: [u8; 76] = [
        0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1d, 0x6a, 0x6f, 0x6e, 0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61,
        0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f, 0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e,
        0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x00, 0x1d, 0x56, 0xbc, 0x2e, 0x01, 0x1e, 0x6a, 0x6f, 0x6e,
        0x61, 0x73, 0x6f, 0x68, 0x6c, 0x61, 0x6e, 0x64, 0x2d, 0x6d, 0x61, 0x63, 0x62, 0x6f, 0x6f,
        0x6b, 0x2d, 0x70, 0x72, 0x6f, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x6c, 0x00, 0x00, 0x00,
        0x00,
    ];

    #[test]
    fn multi_cname_with_padding() {
        let iter = super::SourceDescriptionMessageIterator::try_from(
            MULTI_WITH_AND_WITHOUT_PADDING.as_slice(),
        )
        .unwrap();
        let items = iter.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(items.len(), 2);
        if let SourceDescriptionItemPayload::CNAME(name) = items[0].payload {
            assert_eq!("jonasohland-macbook-pro.local", name);
            assert_eq!(items[0].ssrc, 0x1d56bc2e);
        } else {
            panic!("wrong item returned as payload from item 0")
        }
        if let SourceDescriptionItemPayload::CNAME(name) = items[1].payload {
            assert_eq!("jonasohland-macbook-pro.locall", name);
            assert_eq!(items[1].ssrc, 0x1d56bc2e);
        } else {
            panic!("wrong item returned as payload from item 1")
        }
    }
}
