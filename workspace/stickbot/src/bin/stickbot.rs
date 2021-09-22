use std::io::Result;

use async_std;
use dotenv;
use env_logger;

use stickbot;

fn main() -> Result<()> {
  async_std::task::block_on(async {
    dotenv::dotenv().expect("unable to load environment from '.env'");
    env_logger::init();

    let addr = std::env::var(stickbot::constants::STICKBOT_HTTP_ADDR_ENV).unwrap_or_default();
    log::info!("spawning tide server on {}, connecting services", addr);
    let services = stickbot::Services::new().await?;
    log::info!("services ready, creating application");
    let mut app = tide::with_state(services);

    app.at("/heartbeat").get(stickbot::routes::heartbeat);

    app.at("/auth/start").get(stickbot::routes::auth::start);
    app.at("/auth/complete").get(stickbot::routes::auth::complete);
    app.at("/auth/identify").get(stickbot::routes::auth::identify);

    app.at("/tables").get(stickbot::routes::tables::list);
    app.at("/create-table").get(stickbot::routes::tables::create);

    app.at("/leave-table").post(stickbot::routes::tables::leave);
    app.at("/join-table").post(stickbot::routes::tables::join);

    app.at("/bets").post(stickbot::routes::bets::create);
    app.at("/rolls").post(stickbot::routes::rolls::create);

    app.at("/job").get(stickbot::routes::jobs::find);

    app.at("/admin/drop-tables").get(stickbot::routes::admin::drop_all);
    app.at("/admin/set-balance").get(stickbot::routes::admin::set_balance);

    log::info!("application ready, spawning");
    app.listen(&addr).await?;
    Ok(())
  })
}
