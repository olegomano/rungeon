use std::fmt;
use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct handle_t<T> {
    value: i16,
    _marker: PhantomData<T>,
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
