//! Diagnostic embedding selection; finite bundle loading is shared mechanics.
#[cfg(test)]
pub(super) use crate::browser::assets::Asset;
pub(super) use crate::browser::assets::{Assets, load};

pub(super) fn embedded() -> Assets {
    let entries: &[(&str, &str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/observability_assets.rs"));
    crate::browser::assets::from_entries(entries)
}
