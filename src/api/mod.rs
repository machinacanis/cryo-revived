//! OpenAPI REST client and identity-bound endpoint handles.

mod client;
mod endpoints;

pub use client::{ApiError, Client};
pub use endpoints::{
    ChannelApi, FileInfo, GroupApi, GroupInfo, GroupMember, GuildApi, MemberPage,
    SessionStartLimit, UserApi, WssShard,
};
