pub const AUTHORIZE : &str = "/authorize";
pub const AUTH_CALLBACK : &str = "/auth-callback";

pub const STATUS : &str = "/status";
pub const TOGGLE : &str = "/toggle";
pub const TILES : &str = "/tiles/:zoom";

pub struct StaticDir {
    pub rest_path: &'static str,
    pub file_path: &'static str
}

pub const CONSOLE_DIR: StaticDir = StaticDir{ rest_path: "/console", file_path: "../console/dist" };
pub const TILEMAP_DIR: StaticDir = StaticDir{ rest_path: "/tilemap", file_path: "../tilemap/dist" };
