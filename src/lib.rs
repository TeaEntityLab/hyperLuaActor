extern crate futures;
extern crate hyper;
extern crate url;

extern crate fp_rust;
#[cfg(feature = "for_rlua")]
extern crate lua_actor;
#[cfg(not(feature = "for_rlua"))]
extern crate mlua;
#[cfg(not(feature = "for_rlua"))]
extern crate mlua_actor;
#[cfg(feature = "for_rlua")]
extern crate rlua;

#[cfg(not(feature = "for_rlua"))]
pub mod mlua_bind;
#[cfg(feature = "for_rlua")]
pub mod rlua_bind;
