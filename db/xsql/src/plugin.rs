use log::LevelFilter;
use rbatis::plugin::log::RbatisLogPlugin;

pub fn log_plugin(level_filter: LevelFilter) -> RbatisLogPlugin {
    RbatisLogPlugin { level_filter }
}
