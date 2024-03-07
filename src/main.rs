use std::path::PathBuf;

use analyzer::Analyzer;
use collector::Collector;
use crypto::Granularity;
use rocket::{form::Form, http::Status, response::content::RawHtml, *};

mod analyzer;
mod collector;
mod crypto;
mod web;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(
        r#"
    <form action="/process_crypto" method="POST">
        <label for="crypto">Choose a Crypto:</label>
        <select name="crypto" id="crypto">
            <option value="bitcoin">Bitcoin</option>
            <option value="ethereum">Ethereum</option>
            <option value="dogecoin">Dogecoin</option>
        </select>
        <br>
        <label for="gran">Choose a Granularity:</label>
        <select name="gran" id="gran">
            <option value="onemin">1min</option>
            <option value="fivemin">5min</option>
            <option value="fifteenmin">15min</option>
            <option value="onehr">1hr</option>
            <option value="sixhr">6hr</option>
            <option value="oneday">1day</option>
        </select>
        <br>
        <label for="num">Choose number of candlesticks:</label>
        <select name="num" id="num">
            <option value="10">10</option>
            <option value="25">25</option>
            <option value="50">50</option>
            <option value="100">100</option>
            <option value="200">200</option>
            <option value="300">300</option>
        </select>
        <br>
        <input type ="submit" value="Submit">
    </form>
    "#,
    )
}

pub enum CryptoCoin {
    BTC,
    ETH,
    DOGE,
}

#[derive(FromForm)]
struct UserInput<'a> {
    crypto: &'a str,
    gran: &'a str,
    num: &'a str,
}

struct WebAppState {
    analyzer: Analyzer,
    collector: Collector,
}

#[post("/process_crypto", data = "<input>")]
async fn process_crypto<'a>(
    state: &State<WebAppState>,
    input: Form<UserInput<'a>>,
) -> (Status, RawHtml<String>) {
    let crypto = match input.crypto {
        "bitcoin" => "BTC-USD",
        "ethereum" => "ETH-USD",
        "dogecoin" => "DOGE-USD",
        _ => panic!("unexpected crypto coin chosen"),
    };

    let granularity = match input.gran {
        "onemin" => Granularity::OneMin,
        "fivemin" => Granularity::FiveMin,
        "fifteenmin" => Granularity::FifteenMin,
        "onehr" => Granularity::OneHour,
        "sixhr" => Granularity::SixHours,
        "oneday" => Granularity::OneDay,
        _ => panic!("unexpected granularity chosen"),
    };

    let num_candlesticks = input
        .num
        .parse::<usize>()
        .expect("failed to parse number of candlesticks");

    if let Err(err) = state
        .collector
        .collect(crypto, granularity, num_candlesticks)
        .await
    {
        eprintln!("Error collecting data: {err:?}");
        return (Status::InternalServerError, RawHtml(String::new()));
    }

    match state
        .analyzer
        .analyze(crypto, granularity, num_candlesticks)
    {
        Err(err) => {
            eprintln!("Error analyzing data: {err:?}");
            (Status::InternalServerError, RawHtml(String::new()))
        }
        Ok(html) => (Status::Ok, html),
    }
}

#[launch]
async fn rocket() -> _ {
    let database = std::env::var("WEB_APP_DATABASE").expect("WEB_APP_DATABASE env-var must be set");

    let conn = sqlite::open(&database).expect("failed to open sqlite database");
    conn.execute("CREATE TABLE candles (crypto TEXT, granularity TEXT, open REAL, high REAL, low REAL, close REAL, volume REAL, time INTEGER);")
        .expect("failed to create table");
    drop(conn);

    rocket::build()
        .mount("/", routes![index, process_crypto])
        .manage(WebAppState {
            analyzer: Analyzer::new(PathBuf::from(database.clone())),
            collector: Collector::new(PathBuf::from(database)),
        })
}
