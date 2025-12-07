use once_cell::sync::Lazy;

use super::Cache;
use crate::types::{Key, key::Version};

const TIME_TO_LIVE: u64 = 30;
const TIME_TO_IDLE: u64 = 30;
const SIZE: u64 = 30;

pub static VERSION_CACHE: Lazy<Cache<Version>> =
    Lazy::new(|| Cache::new(TIME_TO_LIVE, TIME_TO_IDLE, Some(SIZE)));

pub static KEY_CACHE: Lazy<Cache<Key>> =
    Lazy::new(|| Cache::new(TIME_TO_LIVE, TIME_TO_IDLE, Some(SIZE)));
