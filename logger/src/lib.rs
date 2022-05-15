mod date_fixed_roller;

pub use json::{array, object};
pub use tracing::{init_jaeger, FutureExt, LocalSpan, MercuryTrace, Span, TRACING_SPAN_TX};
pub use tracing_derive::{tracing, tracing_async};

use date_fixed_roller::DateFixedWindowRoller;

use common::Context;

use json::JsonValue;
use log::{Level, LevelFilter};
use log4rs::append::rolling_file::policy::compound::{trigger::size::SizeTrigger, CompoundPolicy};
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::append::{console::ConsoleAppender, file::FileAppender};
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::{json::JsonEncoder, pattern::PatternEncoder};

use std::collections::HashMap;
use std::path::PathBuf;

// Example
// ```rust
//     let json_obj = json!({
//         "key_01", value_01;
//         "key_02", value_02;
//    });
// ```
#[macro_export]
macro_rules! json {
    ({$($key: expr, $value: expr); *}) => {{
        let mut evt = JsonValue::new_object();
        $(evt[$key] = $value.into();)*
        evt
    }};
}

pub fn init<S: std::hash::BuildHasher>(
    filter: String,
    log_to_console: bool,
    console_show_file_and_line: bool,
    log_to_file: bool,
    metrics: bool,
    log_path: PathBuf,
    file_size_limit: u64, // bytes
    modules_level: HashMap<String, String, S>,
) {
    let console_appender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            if console_show_file_and_line {
                "[{d} {h({l})} {t} {f}:{L}] {m}{n}"
            } else {
                "[{d} {h({l})} {t}] {m}{n}"
            },
        )))
        .build();

    let mercury_roller_pat = log_path.join("{date}.mercury.{timestamp}.log");
    let metrics_roller_pat = log_path.join("{date}.metrics.{timestamp}.log");

    let file_appender = {
        let size_trigger = SizeTrigger::new(file_size_limit);
        let roller = DateFixedWindowRoller::builder()
            .build(&mercury_roller_pat.to_string_lossy())
            .expect("build data fixed window roller");
        let policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));

        RollingFileAppender::builder()
            .encoder(Box::new(JsonEncoder::new()))
            .build(log_path.join("mercury.log"), Box::new(policy))
            .expect("build rolling file appender")
    };

    let cli_file_appender = FileAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build(log_path.join("cli.log"))
        .expect("build file appender");

    let metrics_appender = {
        let size_trigger = SizeTrigger::new(file_size_limit);
        let roller = DateFixedWindowRoller::builder()
            .build(&metrics_roller_pat.to_string_lossy())
            .expect("build data fixed window roller");
        let policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));

        RollingFileAppender::builder()
            .encoder(Box::new(JsonEncoder::new()))
            .build(log_path.join("metrics.log"), Box::new(policy))
            .expect("build rolling file appender")
    };

    let mut root_builder = Root::builder();
    if log_to_console {
        root_builder = root_builder.appender("console");
    }
    if log_to_file {
        root_builder = root_builder.appender("file");
    }

    let level_filter = convert_level(filter.as_ref());
    let root = root_builder.build(level_filter);

    let metrics_logger = Logger::builder().additive(false).appender("metrics").build(
        "metrics",
        if metrics {
            LevelFilter::Trace
        } else {
            LevelFilter::Off
        },
    );

    let cli_logger = Logger::builder()
        .additive(false)
        .appender("cli")
        .appender("console")
        .build("cli", LevelFilter::Trace);

    let mut config_builder = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console_appender)))
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .appender(Appender::builder().build("metrics", Box::new(metrics_appender)))
        .appender(Appender::builder().build("cli", Box::new(cli_file_appender)))
        .logger(metrics_logger)
        .logger(cli_logger);

    for (module, level) in &modules_level {
        let module_logger = Logger::builder()
            .additive(false)
            .appender("console")
            .appender("file")
            .build(module, convert_level(level));
        config_builder = config_builder.logger(module_logger);
    }
    let config = config_builder.build(root).expect("build config");

    log4rs::init_config(config).expect("init config");
}

fn convert_level(level: &str) -> LevelFilter {
    match level {
        "OFF" => LevelFilter::Off,
        "ERROR" => LevelFilter::Error,
        "INFO" => LevelFilter::Info,
        "WARN" => LevelFilter::Warn,
        "DEBUG" => LevelFilter::Debug,
        "TRACE" => LevelFilter::Trace,
        f => {
            println!("invalid logger.filter {}, use INFO", f);
            LevelFilter::Info
        }
    }
}

pub fn metrics(name: &str, mut content: JsonValue) {
    log::trace!(target: "metrics", "{}", {
        content["name"] = name.into();
        content
    });
}

// Usage:
// log(Level::Info, "network", "netw0001", &ctx, common_logger::object!{"music"
// : "beautiful world"})
pub fn log(level: Level, module: &str, event: &str, _ctx: &Context, mut msg: JsonValue) {
    log::log!(target: module, level, "{}", {
        msg["event"] = event.into();
        msg
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::bool_assert_comparison)]
    #[test]
    fn test_json() {
        env_logger::init();
        let json = json!({"height", 1; "msg", "asset_01"; "is_connected", true});
        log(
            Level::Warn,
            "logger",
            "logg_001",
            &Context::new(),
            json.clone(),
        );
        assert_eq!(json["height"], 1);
        assert_eq!(json["msg"], "asset_01");
        assert_eq!(json["is_connected"], true);
    }
}
