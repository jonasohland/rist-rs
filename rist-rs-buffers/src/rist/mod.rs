use crate::reorder::{OrderedPacket, SequenceNumber};

trait IntoOrdered<S: SequenceNumber>: Sized {
    type Ordered: OrderedPacket<S>;
    fn into_ordered(self) -> Result<Self::Ordered, Self>;
}


