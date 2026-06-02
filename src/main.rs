mod auth;
mod battery_capacity;
mod battery_prediction;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod notify;
mod poller;
mod telegram;
mod weather;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    middleware as axum_middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use poller::{PollerStatus, SharedPollerStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "datalogger_backend=debug,tower_http=info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    // ── Database ─────────────────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");

    tracing::info!("Connecting to PostgreSQL...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    tracing::info!("Database connected.");

    tracing::info!("Running migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Migrations complete.");

    // ── JWT secret ────────────────────────────────────────────────────────
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "change-this-secret-in-production-minimum-32-chars".to_string());

    // ── Poller ────────────────────────────────────────────────────────────
    let poller_status: SharedPollerStatus = Arc::new(RwLock::new(PollerStatus::default()));

    // Kombiniraj konfiguracije iz env varijabli i iz baze podataka
    let env_configs = poller::load_configs_from_env();
    let db_configs  = poller::load_configs_from_db(&pool).await;

    // Spoji konfiguracije; env varijable imaju prednost za isti station (po imenu)
    let env_names: std::collections::HashSet<String> = env_configs.iter().map(|c| c.name.clone()).collect();
    let mut all_configs = env_configs;
    for cfg in db_configs {
        if !env_names.contains(&cfg.name) {
            all_configs.push(cfg);
        }
    }

    if all_configs.is_empty() {
        tracing::info!("No pollers configured (no DATALOGGER_URL and no polling_enabled objects in DB) — running in push-only mode");
    } else {
        tracing::info!("Starting {} poller(s)...", all_configs.len());
        poller::start_pollers(all_configs, pool.clone(), poller_status.clone());
    }

    // ── Telegram bot (dvosmjerna komunikacija — upiti) ──────────────────────
    telegram::start_bot(pool.clone());

    // ── CORS ──────────────────────────────────────────────────────────────
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // ── Routes ────────────────────────────────────────────────────────────

    // Push ingest (CR300 → backend) — API key auth
    let push_routes = Router::new()
        .route("/api/v1/datalogger/alarms",       post(handlers::ingest_alarms))
        .route("/api/v1/datalogger/measurements",  post(handlers::ingest_measurements))
        .route("/api/v1/datalogger/eventlogs",     post(handlers::ingest_event_logs))
        .layer(axum_middleware::from_fn_with_state(pool.clone(), middleware::auth_middleware))
        .with_state(pool.clone());

    // Auth (login, refresh, logout — no JWT middleware)
    let auth_routes = Router::new()
        .route("/api/v1/auth/login",   post(handlers::domain::login))
        .route("/api/v1/auth/refresh", post(handlers::domain::refresh_token))
        .route("/api/v1/auth/logout",  post(handlers::domain::logout))
        .layer(axum::Extension(jwt_secret.clone()))
        .with_state(pool.clone());

    // Protected routes — JWT middleware
    let protected = Router::new()
        // Auth/me
        .route("/api/v1/auth/me",                get(handlers::domain::me))
        // Regions
        .route("/api/v1/regions",                get(handlers::domain::list_regions))
        .route("/api/v1/regions",                post(handlers::domain::create_region))
        .route("/api/v1/regions/:id",            put(handlers::domain::update_region))
        .route("/api/v1/regions/summary",        get(handlers::domain::region_summary))
        // Station types
        .route("/api/v1/station-types",          get(handlers::domain::list_station_types))
        // Objects
        .route("/api/v1/objects",                get(handlers::domain::list_objects))
        .route("/api/v1/objects",                post(handlers::domain::create_object))
        .route("/api/v1/objects/:id",            get(handlers::domain::get_object))
        .route("/api/v1/objects/:id",            patch(handlers::domain::update_object))
        .route("/api/v1/objects/:id",            delete(handlers::domain::delete_object))
        // Measurements po objektu
        .route("/api/v1/objects/:id/measurements/10min",  get(handlers::domain::get_measurements_10min))
        .route("/api/v1/objects/:id/measurements/1h",     get(handlers::domain::get_measurements_1h))
        .route("/api/v1/objects/:id/measurements/24h",    get(handlers::domain::get_measurements_24h))
        .route("/api/v1/objects/:id/measurements/latest", get(handlers::domain::get_latest_measurement))
        // Predikcija kvara baterije
        .route("/api/v1/objects/:id/battery/prediction",  get(handlers::domain::predict_battery))
        // Procjena kapaciteta baterije
        .route("/api/v1/objects/:id/battery/capacity",    get(handlers::domain::estimate_battery_capacity))
        // Vremenski uvjeti (Open-Meteo)
        .route("/api/v1/objects/:id/weather",             get(handlers::domain::get_weather))
        // Solarni efikasnost score
        .route("/api/v1/objects/:id/solar-efficiency",    get(handlers::domain::get_solar_efficiency))
        // Globalni alarmi
        .route("/api/v1/alarms",                          get(handlers::domain::list_alarms))
        .route("/api/v1/alarms/:id",                      delete(handlers::domain::delete_alarm))
        // Alarmi po objektu
        .route("/api/v1/objects/:id/alarms",              get(handlers::domain::get_alarms))
        .route("/api/v1/objects/:id/alarms",              delete(handlers::domain::delete_alarms))
        .route("/api/v1/objects/:id/alarms/heatmap",      get(handlers::domain::get_alarm_heatmap))
        .route("/api/v1/objects/:id/alarms/active",       get(handlers::domain::get_active_alarms))
        .route("/api/v1/objects/:id/alarms/acknowledge",  post(handlers::domain::acknowledge_alarm))
        // Event log po objektu
        .route("/api/v1/objects/:id/eventlogs",      get(handlers::domain::get_event_logs))
        // Users (admin only)
        .route("/api/v1/users",                       get(handlers::domain::list_users))
        .route("/api/v1/users",                       post(handlers::domain::create_user))
        .route("/api/v1/users/:id/regions",           get(handlers::domain::get_user_regions))
        .route("/api/v1/users/regions",               post(handlers::domain::grant_region_access))
        .route("/api/v1/users/:uid/regions/:rid",     delete(handlers::domain::revoke_region_access))
        // Audit log (admin only)
        .route("/api/v1/admin/audit-log",             get(handlers::domain::get_audit_log))
        // Obavještavanje (admin only)
        .route("/api/v1/notifications/channels",          get(handlers::notify::list_channels))
        .route("/api/v1/notifications/channels",          post(handlers::notify::create_channel))
        .route("/api/v1/notifications/channels/:id",      patch(handlers::notify::update_channel))
        .route("/api/v1/notifications/channels/:id",      delete(handlers::notify::delete_channel))
        .route("/api/v1/notifications/channels/:id/test", post(handlers::notify::test_channel))
        .route("/api/v1/notifications/rules",             get(handlers::notify::list_rules))
        .route("/api/v1/notifications/rules",             post(handlers::notify::create_rule))
        .route("/api/v1/notifications/rules/:id",         patch(handlers::notify::update_rule))
        .route("/api/v1/notifications/rules/:id",         delete(handlers::notify::delete_rule))
        .route("/api/v1/notifications/log",              get(handlers::notify::list_log))
        // Change password (any authenticated user)
        .route("/api/v1/auth/change-password",        post(handlers::domain::change_password))
        // Poller control
        .route("/api/v1/control/setvalue",            post(handlers::poller_handler::set_datalogger_value))
        .route("/api/v1/objects/:id/poll",            post(handlers::poller_handler::poll_object_now))
        .layer(axum_middleware::from_fn_with_state(jwt_secret.clone(), middleware::jwt_middleware))
        .layer(axum::Extension(jwt_secret.clone()))
        .with_state(pool.clone());

    // Poller status (no auth — internal monitoring)
    let poller_routes = Router::new()
        .route("/api/v1/poller/status", get(handlers::poller_handler::poller_status))
        .with_state(poller_status);

    // Health
    let health = Router::new()
        .route("/health", get(handlers::health))
        .with_state(pool.clone());

    let app = Router::new()
        .merge(push_routes)
        .merge(auth_routes)
        .merge(protected)
        .merge(poller_routes)
        .merge(health)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors);

    let port     = std::env::var("PORT").unwrap_or_else(|_| "8095".to_string());
    let addr     = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("════════════════════════════════════════════");
    tracing::info!("  Datalogger Backend v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("  Listening on {}", addr);
    tracing::info!("════════════════════════════════════════════");
    tracing::info!("  PUSH  POST /api/v1/datalogger/alarms");
    tracing::info!("  PUSH  POST /api/v1/datalogger/measurements");
    tracing::info!("  PUSH  POST /api/v1/datalogger/eventlogs");
    tracing::info!("  AUTH  POST /api/v1/auth/login");
    tracing::info!("  GET   /api/v1/regions");
    tracing::info!("  GET   /api/v1/objects");
    tracing::info!("  GET   /api/v1/objects/:id/measurements/10min");
    tracing::info!("  GET   /api/v1/objects/:id/alarms/active");
    tracing::info!("  GET   /health");
    tracing::info!("════════════════════════════════════════════");

    axum::serve(listener, app).await?;
    Ok(())
}
