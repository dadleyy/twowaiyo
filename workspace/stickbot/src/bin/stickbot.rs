use std::io::Result;

use async_std;
use dotenv;
use env_logger;

use stickbot;

fn main() -> Result<()> {
  /*
  async_std::task::block_on(async {
    let url = std::env::var(stickbot::constants::MONGO_DB_ENV_URL).unwrap_or_default();
    log::debug!("loaded mongo url from env - ({} bytes)", url.len());
    let mongo = stickbot::db::connect(url).await?;
    let db = mongo.database(stickbot::constants::MONGO_DB_DATABASE_NAME);

    let names = db.list_collection_names(None).await.unwrap_or_default();
    log::debug!("collection names - {:?}", names);
    let collection = db.collection::<bankah::TableState>(stickbot::constants::MONGO_DB_TABLE_COLLECTION_NAME);

    collection.drop(None).await.expect("unable to clear");

    let mut table = twowaiyo::Table::default();
    let mut player = twowaiyo::Player::default();
    table = table.sit(&mut player);

    table = table
      .bet(&player, &twowaiyo::Bet::start_pass(10))
      .expect("should be able to.");

    table = table
      .bet(&player, &twowaiyo::Bet::Field(100))
      .expect("should be able to.");

    collection
      .insert_one(bankah::TableState::from(&table), None)
      .await
      .map_err(stickbot::db::mongo_error)?;

    let cursor = collection
      .find_one(stickbot::db::doc! { "id": table.identifier() }, None)
      .await
      .map_err(stickbot::db::mongo_error)?;

    if let Some(state) = cursor {
      log::debug!("loaded state from cursor - {:?}", state);
      log_table(state.into())
    }

    println!("new table - {:?}", table);
    Ok(())
  })
  */

  async_std::task::block_on(async {
    dotenv::dotenv().expect("unable to load environment from '.env'");
    env_logger::init();

    let addr = std::env::var(stickbot::constants::STICKBOT_HTTP_ADDR_ENV).unwrap_or_default();
    log::info!("spawning tide server on {}, connecting services", addr);
    let services = stickbot::Services::new().await?;
    log::info!("services ready, creating application");
    let mut app = tide::with_state(services);
    app.at("/heartbeat").get(stickbot::routes::heartbeat);
    log::info!("application ready, spawning");
    app.listen(&addr).await?;
    Ok(())
  })
}
