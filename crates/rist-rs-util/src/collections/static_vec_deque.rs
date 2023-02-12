use alloc::collections::VecDeque;

pub struct StaticVecDeque<T> {
    max_len: usize,
    data: VecDeque<T>,
}

impl<T> StaticVecDeque<T> {
    pub fn new(max_len: usize) -> Self {
        Self {
            max_len,
            data: VecDeque::<T>::with_capacity(max_len),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.max_len
    }

    pub fn clear(&mut self) {
        self.data.clear()
    }

    pub fn push_back(&mut self, item: T) -> Option<T> {
        if !self.is_full() {
            self.data.push_back(item);
            None
        } else {
            Some(item)
        }
    }

    pub fn push_front(&mut self, item: T) -> Option<T> {
        if !self.is_full() {
            self.data.push_front(item);
            None
        } else {
            Some(item)
        }
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.data.pop_back()
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn front(&mut self) -> Option<&T> {
        self.data.front()
    }

    pub fn back(&mut self) -> Option<&T> {
        self.data.back()
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.data.front_mut()
    }

    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.data.back_mut()
    }
}
