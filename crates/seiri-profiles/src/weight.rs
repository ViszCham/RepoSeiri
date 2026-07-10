use std::num::NonZeroU16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StaticProfileWeight(NonZeroU16);

impl StaticProfileWeight {
    pub fn from_registry_value(value: u16) -> Option<Self> {
        NonZeroU16::new(value).map(Self)
    }

    #[must_use]
    pub fn get(self) -> u32 {
        u32::from(self.0.get())
    }
}
