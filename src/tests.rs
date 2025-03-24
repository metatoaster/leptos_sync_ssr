#[cfg(feature = "ssr")]
use reactive_graph::owner::Owner;

#[cfg(feature = "ssr")]
pub(crate) fn set_reactive_owner() -> Owner {
    let owner = Owner::new();
    owner.set();
    owner
}

#[cfg(feature = "ssr")]
mod ready;
