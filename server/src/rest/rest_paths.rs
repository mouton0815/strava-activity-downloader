pub const AUTHORIZE : &str = "/authorize";
pub const AUTH_CALLBACK : &str = "/auth-callback";

pub const STATUS : &str = "/status";
pub const TOGGLE : &str = "/toggle";
pub const TILES : &str = "/tiles/:zoom";

pub struct StaticDir {
    pub rest_path: &'static str,
    pub file_path: &'static str
}

pub const WEB_DIR: StaticDir = StaticDir{ rest_path: "/console", file_path: "../web/dist" };
pub const MAP_DIR: StaticDir = StaticDir{ rest_path: "/map", file_path: "../map/dist" };
