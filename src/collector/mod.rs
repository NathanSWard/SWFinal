use crate::crypto::{Candle, Granularity};
use rocket::*;
use std::path::PathBuf;

pub struct Collector {
    database: PathBuf,
}

impl Collector {
    pub fn new(database: PathBuf) -> Self {
        Self { database }
    }

    pub async fn collect(
        &self,
        crypto: impl Into<String>,
        granularity: Granularity,
        num_candlesticks: usize,
    ) -> anyhow::Result<()> {
        let crypto = crypto.into();
        let candles = get_candles(&crypto, granularity).await?;
        tokio::task::spawn_blocking({

            let database = self.database.clone();
            move || -> anyhow::Result<()> {
                let conn = sqlite::open(&database)?;
                for Candle {
                    open,
                    high,
                    low,
                    close,
                    time,
                    volume,
                } in candles
                {
                    let time = time.timestamp();
                    conn.execute(format!("INSERT INTO candles VALUES ('{crypto}', '{}', {open}, {high}, {low}, {close}, {volume}, {time});", <&'static str>::from(&granularity)))?;
                }

                Ok(())
            }
        }).await?
    }
}

pub async fn get_candles(crypto: &str, granularity: Granularity) -> anyhow::Result<Vec<Candle>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "https://api.exchange.coinbase.com/products/{crypto}/candles"
        ))
        .query(&[("granularity", granularity.http_query_param())])
        .header("User-Agent", "CSCA5028-final")
        .send()
        .await?;
    Ok(resp.json::<Vec<Candle>>().await?)
}
