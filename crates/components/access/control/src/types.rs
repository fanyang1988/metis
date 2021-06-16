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
use scale_info::TypeInfo;

/// The default `RoleId` type by use byte32.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    Encode,
    Decode,
    Default,
)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
#[derive(SpreadLayout, PackedLayout)]
pub struct RoleId([u8; 32]);

impl<'a> TryFrom<&'a [u8]> for RoleId {
    type Error = TryFromSliceError;

    fn try_from(bytes: &'a [u8]) -> Result<Self, TryFromSliceError> {
        let address = <[u8; 32]>::try_from(bytes)?;
        Ok(Self(address))
    }
}
