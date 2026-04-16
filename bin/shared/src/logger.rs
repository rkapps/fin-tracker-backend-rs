use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn set_logger() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        "fin_services=trace,fin_core=debug,agentic_core=info,fin_tracker_pipeline=info".to_string()
    });

    let is_cloud = std::env::var("LOG_FORMAT").is_ok();

    println!("{}", filter);
    if is_cloud {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .with_target(true)
            .with_line_number(true)
            .with_env_filter(filter)
            .json()
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .with_target(true)
            .with_line_number(true)
            .with_env_filter(filter)
            .compact()
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    };
}
