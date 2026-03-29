/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    None,
    Unknown,
    Player,
    Studio,
    StudioAuth,
}

///
/// havent tested studio at all so uh if it doesnt work then its goy
/// 

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchFlag {
    pub identifiers: &'static [&'static str],
    pub active: bool,
    pub data: Option<String>,
}

impl LaunchFlag {
    pub const fn new(identifiers: &'static [&'static str]) -> Self {
        Self {
            identifiers,
            active: false,
            data: None,
        }
    }

    pub fn mark_active(&mut self, data: Option<String>) {
        self.active = true;
        self.data = data;
    }
}
