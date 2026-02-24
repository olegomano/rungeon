use std::fmt;
use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug)]
pub struct handle_t<T> {
    value: i16,
    _marker: PhantomData<T>,
}

impl<T> Copy for handle_t<T> {}
impl<T> Clone for handle_t<T> {
    fn clone(&self) -> handle_t<T> {
        return *self;
    }
}

impl<T> handle_t<T> {
    pub fn new(value: i16) -> Self {
        return Self {
            value: value,
            _marker: PhantomData,
        };
    }

    /*
     * gen: top 3 bits are the genrationn index
     * g_index: next 8 bits is the node index
     * i_index: last 5 bits is the instance indexx
     */
    pub fn from(gen: u8, g_index: u8, i_index: u8) -> Self {
        let mut value = 0;
        value = (gen as i16) << 13;
        value |= (g_index as i16) << 5;
        value |= i_index as i16;
        return Self::new(value);
    }

    pub fn null() -> Self {
        return Self::new(0);
    }

    pub fn IsNull(&self) -> bool {
        return self.value == 0;
    }

    /*
     * Retuns the generation of handle
     */
    pub fn Generation(&self) -> u8 {
        return (self.value >> 13) as u8;
    }

    /*
     * Returns the index of the node
     */
    pub fn Node(&self) -> u8 {
        return ((self.value >> 5) & 0xff) as u8;
    }

    pub fn Instance(&self) -> u8 {
        return (self.value & 0x7F) as u8;
    }

    pub fn Value(&self) -> i16 {
        return self.value;
    }
}

impl<T> fmt::Display for handle_t<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "Node: {} Index: {}", self.Node(), self.Instance());
    }
}

impl<T> Default for handle_t<T> {
    fn default() -> Self {
        Self::null()
    }
}

// Implement equality/ordering/hash manually so they are based only on the
// internal `value` and do not impose trait bounds on `T`.
impl<T> PartialEq for handle_t<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for handle_t<T> {}

impl<T> PartialOrd for handle_t<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for handle_t<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> std::hash::Hash for handle_t<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
