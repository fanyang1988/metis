use scale::{
    Decode,
    Encode,
};

use core::{
    array::TryFromSliceError,
    convert::TryFrom,
};

use ink_storage::traits::{
    PackedLayout,
    SpreadLayout,
};

#[cfg(feature = "std")]
use ink_storage::traits::StorageLayout;

#[cfg(feature = "std")]
use scale_info::TypeInfo;

/// The default `TokenId` type by use u256 like.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Encode, Decode, Default,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
#[derive(SpreadLayout, PackedLayout)]
pub struct TokenId([u8; 32]);

impl<'a> TryFrom<&'a [u8]> for TokenId {
    type Error = TryFromSliceError;

    fn try_from(bytes: &'a [u8]) -> core::result::Result<Self, TryFromSliceError> {
        let address = <[u8; 32]>::try_from(bytes)?;
        Ok(Self(address))
    }
}

impl TokenId {
    pub const fn new(data: [u8; 32]) -> Self {
        Self(data)
    }

    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}
